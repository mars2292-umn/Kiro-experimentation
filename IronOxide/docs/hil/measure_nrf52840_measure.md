# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:4b344c1a0307f3d124a9d53c46bc5823fb8ccb5fbc56ce8a479d4295bf305f0c

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| EasyDMA footprint (HW-15) at 8 Mbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 1 Mbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 125 kbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 8 Mbps on SPIM1 | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 194 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5803 cycles (90.7 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17532 cycles (273.9 µs) over 5167231 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10751 rises and 10000 response edges in the capture); latency 100 to 4880 µs, median 110 µs, 0 later than 15 ms; Δ_stamp observed 4148 cycles (64.8 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.2 % of the run, the Kernel 33.9 % over 1150533 entries; runs per Task [18027, 6003, 20997, 5070, 9999, 7220, 1827] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 29273 cycles (0.457 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 28160 cycles (0.440 ms at 100000 samples/s; 18027 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 409 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 248 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5803 / 90.7 | 3564 | 2^12:17747 2^13:252 |
| 2 | 8999 | 5161 / 80.6 | 3245 | 2^12:8997 2^13:2 |
| 3 | 4499 | 4073 / 63.6 | 3577 | 2^12:4499 |
| 5 | 7199 | 4383 / 68.5 | 3596 | 2^12:6951 2^13:248 |
| 6 | 1799 | 4073 / 63.6 | 3887 | 2^12:1799 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.767 | 0 | 0 | 0 |
| 1 | 5999 | 0.257 | 0 | 0 | 0 |
| 2 | 8999 | 5.921 | 0 | 0 | 0 |
| 3 | 4500 | 4.328 | 0 | 0 | 0 |
| 4 | 9999 | 4.888 | 10000 | 2273 | 2017 |
| 5 | 7200 | 6.356 | 0 | 0 | 0 |
| 6 | 1800 | 4.878 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 29273 / 0.457 (min 17073, mean 19788) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7389 / 0.115 (min 3473, mean 3619) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 343169 / 5.362 (min 336397, mean 337938) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 224458 / 3.507 (min 203662, mean 212178) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 7328 / 0.115 (min 3556, mean 3965) | 0 above the Budget; 2^12:8997 2^13:1002 |
| Task 5 entry | 7200 | 23379 / 0.365 (min 9676, mean 13220) | 0 above the Budget; 2^14:5293 2^15:1907 |
| Task 6 entry | 1800 | 25776 / 0.403 (min 8021, mean 11008) | 0 above the Budget; 2^13:385 2^14:1383 2^15:32 |
| Task 0 on Resource 0 | 18000 | 7228 / 0.113 | |
| Task 0 on Resource 1 | 18000 | 7542 / 0.118 | |
| Task 2 on Resource 0 | 8999 | 325655 / 5.088 | |
| Task 2 on Resource 1 | 17998 | 4146 / 0.065 | |
| Task 5 on Resource 2 | 7200 | 4164 / 0.065 | |
| Task 6 on Resource 2 | 1800 | 5477 / 0.086 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3252 / 50.81 | 3027 |
| timer | 49595 | 3817 / 59.64 | 2966 |
| intake | 10000 | 2399 / 37.48 | 2148 |
| lock | 71997 | 466 / 7.28 | 464 |
| unlock | 71997 | 1875 / 29.30 | 1622 |
| job_end | 56497 | 2085 / 32.58 | 1727 |
| now | 4785791 | 236 / 3.69 | 235 |
| raise | 6000 | 2103 / 32.86 | 1985 |
| dma | 1855 | 742 / 11.59 | 535 |
| epilogue | 56497 | 1254 / 19.59 | 607 |
| view | 69114 | 329 / 5.14 | 297 |
| send | 44552 | 2316 / 36.19 | 2148 |
| recv | 90671 | 957 / 14.95 | 861 |

Entries that interrupted an executing Job: Task 0: 4770, Task 1: 2, Task 2: 27688, Task 3: 2385, Task 4: 1002, Task 5: 920, Task 6: 1470

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8156 cycles; Task 1: 1 (timer), 2930 cycles; Task 2: 5 (dispatch, timer, intake), 14007 cycles; Task 3: 4 (dispatch, timer, intake), 11009 cycles; Task 4: 1 (timer), 2934 cycles; Task 5: 2 (dispatch, intake), 5517 cycles; Task 6: 3 (dispatch, timer, intake), 8259 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3751 cycles
- Task 0 at 179940.000 ms, 3193 cycles
- Task 2 at 179945.000 ms, 3197 cycles
- Task 0 at 179950.000 ms, 4087 cycles
- Task 5 at 179950.000 ms, 4087 cycles
- Task 0 at 179960.000 ms, 3429 cycles
- Task 3 at 179960.000 ms, 3429 cycles
- Task 2 at 179965.000 ms, 3197 cycles
- Task 0 at 179970.000 ms, 3663 cycles
- Task 5 at 179975.000 ms, 3201 cycles
- Task 0 at 179980.000 ms, 3193 cycles
- Task 0 at 179990.000 ms, 3081 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5167231 of them, the longest 17532 cycles (273.9 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4774584 |
| 512 to 1023 (8.0 to 16.0 µs) | 72974 |
| 1024 to 2047 (16.0 to 32.0 µs) | 149349 |
| 2048 to 4095 (32.0 to 64.0 µs) | 100948 |
| 4096 to 8191 (64.0 to 128.0 µs) | 63639 |
| 8192 to 16383 (128.0 to 256.0 µs) | 5673 |
| 16384 to 32767 (256.0 to 512.0 µs) | 64 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 53807 | 17532 / 273.9 |
| timer | 32093 | 14399 / 225.0 |
| job_end | 5048011 | 4851 / 75.8 |
| epilogue | 33320 | 3991 / 62.4 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 526323 | 18027 | 0.440 |
| 1 | 24254 | 6003 | 0.100 |
| 2 | 4803306 | 20997 | 5.250 |
| 3 | 1486268 | 5070 | 3.380 |
| 4 | 44163 | 9999 | 0.100 |
| 5 | 135291 | 7220 | 0.330 |
| 6 | 27694 | 1827 | 0.370 |
| none | 10949262 | 43873 | 9.510 |

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
relay_reqs=2250
relay_rounds=2250
report_reqs=1125
report_rounds=1125
triangle_fwd=3600
triangle_acks=3600
triangle_rounds=3600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=409
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=248
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=1799
dma_after_stop=0
dma_footprint=1
dma_footprint_1m=1
dma_footprint_125k=1
dma_footprint_spim1=1
spim0_enable=7
spim0_shorts=131072
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
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
     Finished in 8.79s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json, docs/hil/kernel_timing_nrf52840_response.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:4b344c1a0307f3d124a9d53c46bc5823fb8ccb5fbc56ce8a479d4295bf305f0c) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:16ef3ddeff8b449deecb26c19f66902614bdbbb3c80e2d4d29ecb9681350291e is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,response)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 583 cycles (measured, rsk-hil release-timing record, 71997 runs, max 466 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2372 cycles (measured, rsk-hil release-timing record, 714 runs, max 1897 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4103 cycles (measured, rsk-hil release-timing record, 555 runs, max 3282 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4177 cycles (measured, rsk-hil release-timing record, 550 runs, max 3341 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 5605 cycles (measured, rsk-hil release-timing record, 529 runs, max 4484 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 5185 cycles (measured, rsk-hil release-timing record, 10000 runs, max 4148 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1037 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 21915 cycles (measured, rsk-hil release-timing record, 5167231 runs, max 17532 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 17532 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 53807 (longest 17532), timer 32093 (longest 14399), job_end 5048011 (longest 4851), epilogue 33320 (longest 3991); the longest single Rust half 3817) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 134 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2095, less Δ_entry of the plain cycle configuration, 2069, is 26, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 134 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 134 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2095, less Δ_entry of the plain cycle configuration, 2069, is 26, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 134 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 5605 cycles (measured, rsk-hil release-timing record, 529 runs, max 4484 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 5605 cycles (measured, rsk-hil release-timing record, 4 runs, max 4484 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_hm: 3297 cycles (measured, rsk-hil release-timing record, 3 runs, max 2637 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one fault entry applying the Partition's fault response (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_wake: 4320 cycles (measured, rsk-hil release-timing record, 5719 runs, max 3456 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from a Release_Source edge that arrived with no Job executing (the processor in the idle activity's WFI), latched by the time base through PPI, to the intake handler's timestamp: the wake-up from sleep, with the exception entry and the handler's read of the time base after it (an upper bound of the wake-up alone)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 7254 cycles (measured, rsk-hil release-timing record, 40495 runs, max 5803 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 295 cycles (measured, rsk-hil release-timing record, 4785791 runs, max 236 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 928 cycles (measured, rsk-hil release-timing record, 1855 runs, max 742 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 706 runs, max 329 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 5605 cycles (measured, rsk-hil release-timing record, 3 runs, max 4484 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stop: 5414 cycles (measured, rsk-hil release-timing record, 1 runs, max 4331 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: an entry that stopped a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2587 cycles (measured, rsk-hil release-timing record, 163 runs, max 2069 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 163 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2947 cycles (measured, rsk-hil release-timing record, 277 runs, max 2357 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1197 cycles (measured, rsk-hil release-timing record, 90671 runs, max 957 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 102 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 0.000 | 10.000 | 0.767 | 18000 | not schedulable: no WCRT to hold |
| `part0.echo` | A | 0.000 | 20.000 | 0.257 | 5999 | not schedulable: no WCRT to hold |
| `part0.sensor` | A | 0.000 | 20.000 | 5.921 | 8999 | not schedulable: no WCRT to hold |
| `part1.log` | C | 0.000 | 40.000 | 4.328 | 4500 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 0.000 | 15.000 | 4.888 | 9999 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 0.000 | 25.000 | 6.356 | 7200 | not schedulable: no WCRT to hold |
| `part2.sampler` | B | 0.000 | 100.000 | 4.878 | 1800 | not schedulable: no WCRT to hold |

The plain configuration's analysis is not a PASS: the schedulability claim under measured timing does not hold for this set.

Verdict: **FAIL**
