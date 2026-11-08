# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6230 cycles (97.3 µs) against J_rel 12500 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16427 cycles (256.7 µs) over 6400113 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10680 rises and 10000 response edges in the capture); latency 90 to 5690 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 3740 cycles (58.4 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 43.0 % of the run, the Kernel 39.5 % over 679699 entries; runs per Task [18038, 6001, 20997, 5249, 9998, 7246, 1813] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 25127 cycles (0.393 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 24320 cycles (0.380 ms at 100000 samples/s; 18038 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 542 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 274 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6230 / 97.3 | 3537 | 2^12:17895 2^13:104 |
| 2 | 8999 | 4326 / 67.6 | 3288 | 2^12:8997 2^13:2 |
| 3 | 4499 | 6230 / 97.3 | 3617 | 2^12:4398 2^13:101 |
| 5 | 7199 | 4114 / 64.3 | 3609 | 2^12:7099 2^13:100 |
| 6 | 1799 | 4114 / 64.3 | 3921 | 2^12:1699 2^13:100 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.671 | 0 | 0 | 0 |
| 1 | 5999 | 0.247 | 0 | 0 | 0 |
| 2 | 8999 | 6.891 | 0 | 0 | 0 |
| 3 | 4500 | 4.033 | 0 | 0 | 0 |
| 4 | 9999 | 5.711 | 10000 | 2174 | 2009 |
| 5 | 7200 | 7.130 | 0 | 0 | 0 |
| 6 | 1800 | 4.341 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 25127 / 0.393 (min 13330, mean 16980) | 0 above the Budget; 2^14:6480 2^15:11520 |
| Task 1 entry | 5999 | 6932 / 0.108 (min 3159, mean 3264) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 407526 / 6.368 (min 399410, mean 401276) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 213386 / 3.334 (min 198770, mean 204554) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 6975 / 0.109 (min 3251, mean 3289) | 0 above the Budget; 2^12:9988 2^13:11 |
| Task 5 entry | 7200 | 11486 / 0.179 (min 6419, mean 6528) | 0 above the Budget; 2^13:7144 2^14:56 |
| Task 6 entry | 1800 | 19250 / 0.301 (min 7426, mean 9753) | 0 above the Budget; 2^13:665 2^14:1134 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7055 / 0.110 | |
| Task 0 on Resource 1 | 18000 | 4237 / 0.066 | |
| Task 2 on Resource 0 | 8999 | 388840 / 6.076 | |
| Task 2 on Resource 1 | 17998 | 4126 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 5299 / 0.083 | |
| Task 6 on Resource 2 | 1800 | 4039 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2881 / 45.02 | 2730 |
| timer | 55643 | 3869 / 60.45 | 3035 |
| intake | 10000 | 2294 / 35.84 | 2127 |
| lock | 71997 | 468 / 7.31 | 467 |
| unlock | 71997 | 1805 / 28.20 | 1637 |
| job_end | 56497 | 2059 / 32.17 | 1787 |
| now | 6093972 | 240 / 3.75 | 240 |
| raise | 6000 | 2094 / 32.72 | 1988 |
| dma | 1808 | 733 / 11.45 | 512 |
| epilogue | 56497 | 1209 / 18.89 | 576 |
| view | 69306 | 329 / 5.14 | 294 |
| send | 27001 | 2166 / 33.84 | 2017 |
| recv | 35997 | 751 / 11.73 | 698 |

Entries that interrupted an executing Job: Task 0: 11584, Task 1: 2, Task 2: 29940, Task 3: 2552, Task 4: 11, Task 5: 103, Task 6: 1149

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3604 cycles
- Task 0 at 179940.000 ms, 3242 cycles
- Task 2 at 179945.000 ms, 3238 cycles
- Task 0 at 179950.000 ms, 3916 cycles
- Task 5 at 179950.000 ms, 3916 cycles
- Task 0 at 179960.000 ms, 3470 cycles
- Task 3 at 179960.000 ms, 3470 cycles
- Task 2 at 179965.000 ms, 3238 cycles
- Task 0 at 179970.000 ms, 3604 cycles
- Task 5 at 179975.000 ms, 3226 cycles
- Task 0 at 179980.000 ms, 3242 cycles
- Task 0 at 179990.000 ms, 3134 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 6400113 of them, the longest 16427 cycles (256.7 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 6079891 |
| 512 to 1023 (8.0 to 16.0 µs) | 109245 |
| 1024 to 2047 (16.0 to 32.0 µs) | 55888 |
| 2048 to 4095 (32.0 to 64.0 µs) | 84194 |
| 4096 to 8191 (64.0 to 128.0 µs) | 58636 |
| 8192 to 16383 (128.0 to 256.0 µs) | 12258 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 47534 | 16427 / 256.7 |
| timer | 37550 | 13193 / 206.1 |
| job_end | 6282122 | 5545 / 86.6 |
| epilogue | 32907 | 3883 / 60.7 |

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 455629 | 18038 | 0.380 |
| 1 | 23409 | 6001 | 0.100 |
| 2 | 5690965 | 20997 | 6.240 |
| 3 | 1435248 | 5249 | 3.250 |
| 4 | 37555 | 9998 | 0.100 |
| 5 | 63619 | 7246 | 0.160 |
| 6 | 24888 | 1813 | 0.170 |
| none | 10265318 | 43681 | 8.550 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/measure/nrf52840_measure/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/measure/nrf52840_measure/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=4500
pulse=9999
echo=5999
blocked_releases=8999
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
telemetry_rx=17997
telemetry_last=17996
telemetry_full=0
pingpong_reps=4500
pingpong_last=17994
pingpong_reqs=4500
pingpong_rejected=0
pingpong_peer=0
telemetry_peer=0
protocol_violations=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=542
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=274
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=1799
dma_after_stop=0
spim0_enable=7
spim0_shorts=131072
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
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
RSK-OK
Firmware exited successfully
     Finished in 7.76s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:471b198131cb9aaacc99213d36f21c30d050d2697ea1cac69d867362fb5481b5) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:7107bc3df75cb35d0728869b8e643cc87dbf300fc64768a39262fe5bb5c6ea43 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 589 cycles (measured, rsk-hil release-timing record, 317 runs, max 471 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2313 cycles (measured, rsk-hil release-timing record, 317 runs, max 1850 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3602 cycles (measured, rsk-hil release-timing record, 56497 runs, max 2881 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4085 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3268 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4837 cycles (measured, rsk-hil release-timing record, 55643 runs, max 3869 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 4675 cycles (measured, rsk-hil release-timing record, 10000 runs, max 3740 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (935 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 20534 cycles (measured, rsk-hil release-timing record, 6400113 runs, max 16427 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 16427 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 47534 (longest 16427), timer 37550 (longest 13193), job_end 6282122 (longest 5545), epilogue 32907 (longest 3883); the longest single Rust half 3869) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 130 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1961, less Δ_entry of the plain cycle configuration, 2022, is 0, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 130 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 130 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1961, less Δ_entry of the plain cycle configuration, 2022, is 0, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 130 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4837 cycles (measured, rsk-hil release-timing record, 55643 runs, max 3869 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 7788 cycles (measured, rsk-hil release-timing record, 40495 runs, max 6230 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 300 cycles (measured, rsk-hil release-timing record, 6093972 runs, max 240 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 917 cycles (measured, rsk-hil release-timing record, 1808 runs, max 733 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 702 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2528 cycles (measured, rsk-hil release-timing record, 149 runs, max 2022 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 149 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2708 cycles (measured, rsk-hil release-timing record, 27001 runs, max 2166 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 939 cycles (measured, rsk-hil release-timing record, 35997 runs, max 751 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 95 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.303 | 10.000 | 1.671 | 18000 | within |
| `part0.echo` | A | 11.656 | 20.000 | 0.247 | 5999 | within |
| `part0.sensor` | A | 13.126 | 20.000 | 6.891 | 8999 | within |
| `part1.log` | C | 38.859 | 40.000 | 4.033 | 4500 | within |
| `part1.pulse` | C | 14.101 | 15.000 | 5.711 | 9999 | within |
| `part2.filter` | B | 16.490 | 25.000 | 7.130 | 7200 | within |
| `part2.sampler` | B | 39.670 | 100.000 | 4.341 | 1800 | within |

Verdict: **pass**
