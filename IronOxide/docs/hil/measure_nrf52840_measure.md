# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.3 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| EasyDMA footprint (HW-15) at 8 Mbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 1 Mbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 125 kbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 194 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5551 cycles (86.7 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17620 cycles (275.3 µs) over 5353309 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10784 rises and 10000 response edges in the capture); latency 100 to 4600 µs, median 110 µs, 0 later than 15 ms; Δ_stamp observed 4356 cycles (68.1 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.2 % of the run, the Kernel 34.3 % over 956977 entries; runs per Task [18033, 6001, 20996, 5340, 9999, 7335, 1809] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 29062 cycles (0.454 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 28160 cycles (0.440 ms at 100000 samples/s; 18033 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 424 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 259 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:442 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5551 / 86.7 | 3617 | 2^12:16546 2^13:1453 |
| 2 | 8999 | 5450 / 85.2 | 3332 | 2^12:8998 2^13:1 |
| 3 | 4499 | 5050 / 78.9 | 3673 | 2^12:3799 2^13:700 |
| 5 | 7199 | 4407 / 68.9 | 3675 | 2^12:6100 2^13:1099 |
| 6 | 1799 | 4170 / 65.2 | 3971 | 2^12:1100 2^13:699 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.773 | 0 | 0 | 0 |
| 1 | 5999 | 0.257 | 0 | 0 | 0 |
| 2 | 8999 | 5.926 | 0 | 0 | 0 |
| 3 | 4500 | 4.336 | 0 | 0 | 0 |
| 4 | 9999 | 4.621 | 10000 | 2270 | 2077 |
| 5 | 7200 | 6.303 | 0 | 0 | 0 |
| 6 | 1800 | 4.710 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 29062 / 0.454 (min 16982, mean 19594) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 3674 / 0.057 (min 3426, mean 3558) | 0 above the Budget; 2^12:5999 |
| Task 2 entry | 8999 | 343982 / 5.375 (min 336410, mean 337870) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 221225 / 3.457 (min 203389, mean 212241) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 7392 / 0.116 (min 3512, mean 3928) | 0 above the Budget; 2^12:9002 2^13:997 |
| Task 5 entry | 7200 | 25535 / 0.399 (min 9543, mean 13191) | 0 above the Budget; 2^14:5257 2^15:1943 |
| Task 6 entry | 1800 | 22255 / 0.348 (min 7927, mean 10718) | 0 above the Budget; 2^13:486 2^14:1310 2^15:4 |
| Task 0 on Resource 0 | 18000 | 7301 / 0.114 | |
| Task 0 on Resource 1 | 18000 | 7207 / 0.113 | |
| Task 2 on Resource 0 | 8999 | 325318 / 5.083 | |
| Task 2 on Resource 1 | 17998 | 3477 / 0.054 | |
| Task 5 on Resource 2 | 7200 | 5617 / 0.088 | |
| Task 6 on Resource 2 | 1800 | 5418 / 0.085 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3194 / 49.91 | 2968 |
| timer | 49494 | 3916 / 61.19 | 3078 |
| intake | 10000 | 2405 / 37.58 | 2210 |
| lock | 71997 | 462 / 7.22 | 460 |
| unlock | 71997 | 1893 / 29.58 | 1686 |
| job_end | 56497 | 2114 / 33.03 | 1807 |
| now | 4967940 | 224 / 3.50 | 223 |
| raise | 6000 | 2172 / 33.94 | 2057 |
| dma | 1835 | 719 / 11.23 | 504 |
| epilogue | 56497 | 1235 / 19.30 | 605 |
| view | 69479 | 320 / 5.00 | 284 |
| send | 44552 | 2388 / 37.31 | 2228 |
| recv | 90671 | 930 / 14.53 | 830 |

Entries that interrupted an executing Job: Task 0: 3596, Task 2: 27840, Task 3: 2896, Task 4: 997, Task 5: 1072, Task 6: 1324

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8238 cycles; Task 1: none; Task 2: 5 (dispatch, timer, intake), 14734 cycles; Task 3: 3 (dispatch, timer, intake), 8340 cycles; Task 4: 1 (timer), 3063 cycles; Task 5: 2 (timer), 5898 cycles; Task 6: 1 (timer), 2999 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3519 cycles
- Task 0 at 179940.000 ms, 3294 cycles
- Task 2 at 179945.000 ms, 3282 cycles
- Task 0 at 179950.000 ms, 3747 cycles
- Task 5 at 179950.000 ms, 3747 cycles
- Task 0 at 179960.000 ms, 3594 cycles
- Task 3 at 179960.000 ms, 3594 cycles
- Task 2 at 179965.000 ms, 3282 cycles
- Task 0 at 179970.000 ms, 3475 cycles
- Task 5 at 179975.000 ms, 3294 cycles
- Task 0 at 179980.000 ms, 3362 cycles
- Task 0 at 179990.000 ms, 3218 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5353309 of them, the longest 17620 cycles (275.3 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4960372 |
| 512 to 1023 (8.0 to 16.0 µs) | 119312 |
| 1024 to 2047 (16.0 to 32.0 µs) | 91562 |
| 2048 to 4095 (32.0 to 64.0 µs) | 113390 |
| 4096 to 8191 (64.0 to 128.0 µs) | 62814 |
| 8192 to 16383 (128.0 to 256.0 µs) | 5811 |
| 16384 to 32767 (256.0 to 512.0 µs) | 48 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 54077 | 17620 / 275.3 |
| timer | 30937 | 13766 / 215.1 |
| job_end | 5234632 | 4709 / 73.6 |
| epilogue | 33663 | 4055 / 63.4 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 524501 | 18033 | 0.440 |
| 1 | 24866 | 6001 | 0.050 |
| 2 | 4802747 | 20996 | 5.250 |
| 3 | 1489105 | 5340 | 3.420 |
| 4 | 45521 | 9999 | 0.110 |
| 5 | 137316 | 7335 | 0.380 |
| 6 | 27186 | 1809 | 0.330 |
| none | 10945082 | 43509 | 9.480 |

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
loop:part0:sensor:hold=424
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=259
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=1799
dma_after_stop=0
dma_footprint=1
dma_footprint_1m=1
dma_footprint_125k=1
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
     Finished in 8.85s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:b884ee5b46f827d348bddf78e675a54b2d95975b0f7802b508fd792d5bc7c2a3) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:d4faae210fcad55506e286ec2510e80a1398acb827111fa97f1d4dd36f9080fc is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 578 cycles (measured, rsk-hil release-timing record, 71997 runs, max 462 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2377 cycles (measured, rsk-hil release-timing record, 794 runs, max 1901 +25%, scenarios: nrf52840_cycles (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4033 cycles (measured, rsk-hil release-timing record, 1054 runs, max 3226 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4187 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3349 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4895 cycles (measured, rsk-hil release-timing record, 49494 runs, max 3916 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 5445 cycles (measured, rsk-hil release-timing record, 10000 runs, max 4356 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1089 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 22025 cycles (measured, rsk-hil release-timing record, 5353309 runs, max 17620 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 17620 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 54077 (longest 17620), timer 30937 (longest 13766), job_end 5234632 (longest 4709), epilogue 33663 (longest 4055); the longest single Rust half 3916) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4895 cycles (measured, rsk-hil release-timing record, 49494 runs, max 3916 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 6939 cycles (measured, rsk-hil release-timing record, 40495 runs, max 5551 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 284 cycles (measured, rsk-hil release-timing record, 98606 runs, max 227 +25%, scenarios: nrf52840_cycles (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 899 cycles (measured, rsk-hil release-timing record, 1835 runs, max 719 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 1404 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2630 cycles (measured, rsk-hil release-timing record, 317 runs, max 2104 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 157 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2985 cycles (measured, rsk-hil release-timing record, 44552 runs, max 2388 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1163 cycles (measured, rsk-hil release-timing record, 90671 runs, max 930 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 96 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 8.978 | 10.000 | 0.773 | 18000 | within |
| `part0.echo` | A | 9.780 | 20.000 | 0.257 | 5999 | within |
| `part0.sensor` | A | 13.097 | 20.000 | 5.926 | 8999 | within |
| `part1.log` | C | 39.463 | 40.000 | 4.336 | 4500 | within |
| `part1.pulse` | C | 14.098 | 15.000 | 4.621 | 9999 | within |
| `part2.filter` | B | 16.915 | 25.000 | 6.303 | 7200 | within |
| `part2.sampler` | B | 78.964 | 100.000 | 4.710 | 1800 | within |

Verdict: **pass**
