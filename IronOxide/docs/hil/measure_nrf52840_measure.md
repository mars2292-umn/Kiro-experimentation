# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

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
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6725 cycles (105.1 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 14485 cycles (226.3 µs) over 5256014 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10765 rises and 10000 response edges in the capture); latency 100 to 5910 µs, median 110 µs, 0 later than 15 ms; Δ_stamp observed 4604 cycles (71.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.2 % of the run, the Kernel 33.8 % over 1048806 entries; runs per Task [18028, 6000, 20996, 5500, 9998, 7304, 1799] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 30579 cycles (0.478 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 29440 cycles (0.460 ms at 100000 samples/s; 18028 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 417 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 253 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:506 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6725 / 105.1 | 3687 | 2^12:16825 2^13:1174 |
| 2 | 8999 | 6325 / 98.8 | 3321 | 2^12:8938 2^13:61 |
| 3 | 4499 | 6725 / 105.1 | 3640 | 2^12:4383 2^13:116 |
| 5 | 7199 | 6725 / 105.1 | 3712 | 2^12:6031 2^13:1168 |
| 6 | 1799 | 6725 / 105.1 | 3951 | 2^12:1691 2^13:108 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.775 | 0 | 0 | 0 |
| 1 | 5999 | 0.258 | 0 | 0 | 0 |
| 2 | 8999 | 5.930 | 0 | 0 | 0 |
| 3 | 4500 | 4.338 | 0 | 0 | 0 |
| 4 | 9999 | 5.936 | 10000 | 2270 | 2081 |
| 5 | 7200 | 6.351 | 0 | 0 | 0 |
| 6 | 1800 | 4.718 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 30579 / 0.478 (min 17183, mean 20011) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7211 / 0.113 (min 3535, mean 3658) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 343879 / 5.373 (min 336415, mean 337984) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 217362 / 3.396 (min 203122, mean 211285) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10716 / 0.167 (min 3596, mean 4004) | 0 above the Budget; 2^12:8998 2^13:1000 2^14:1 |
| Task 5 entry | 7200 | 27424 / 0.428 (min 9720, mean 13347) | 0 above the Budget; 2^14:5179 2^15:2021 |
| Task 6 entry | 1800 | 26664 / 0.417 (min 8104, mean 11017) | 0 above the Budget; 2^13:420 2^14:1374 2^15:6 |
| Task 0 on Resource 0 | 18000 | 8183 / 0.128 | |
| Task 0 on Resource 1 | 18000 | 7889 / 0.123 | |
| Task 2 on Resource 0 | 8999 | 325744 / 5.090 | |
| Task 2 on Resource 1 | 17998 | 3497 / 0.055 | |
| Task 5 on Resource 2 | 7200 | 5601 / 0.088 | |
| Task 6 on Resource 2 | 1800 | 4198 / 0.066 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3284 / 51.31 | 3061 |
| timer | 49332 | 4012 / 62.69 | 3037 |
| intake | 10000 | 2393 / 37.39 | 2206 |
| lock | 71997 | 457 / 7.14 | 455 |
| unlock | 71997 | 1896 / 29.62 | 1684 |
| job_end | 56497 | 2055 / 32.11 | 1797 |
| now | 4877646 | 228 / 3.56 | 227 |
| raise | 6000 | 2109 / 32.95 | 2036 |
| dma | 1855 | 711 / 11.11 | 544 |
| epilogue | 56497 | 1237 / 19.33 | 607 |
| view | 69599 | 320 / 5.00 | 288 |
| send | 44552 | 2387 / 37.30 | 2223 |
| recv | 90671 | 940 / 14.69 | 847 |

Entries that interrupted an executing Job: Task 0: 4641, Task 1: 2, Task 2: 27775, Task 3: 2754, Task 4: 1002, Task 5: 1015, Task 6: 1398

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer), 9119 cycles; Task 1: 1 (timer), 2885 cycles; Task 2: 4 (dispatch, timer), 12048 cycles; Task 3: 1 (timer), 2897 cycles; Task 4: 2 (timer), 5674 cycles; Task 5: 3 (dispatch, timer, intake), 8496 cycles; Task 6: 1 (timer), 2981 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3798 cycles
- Task 0 at 179940.000 ms, 3317 cycles
- Task 2 at 179945.000 ms, 3249 cycles
- Task 0 at 179950.000 ms, 4142 cycles
- Task 5 at 179950.000 ms, 4142 cycles
- Task 0 at 179960.000 ms, 3493 cycles
- Task 3 at 179960.000 ms, 3493 cycles
- Task 2 at 179965.000 ms, 3249 cycles
- Task 0 at 179970.000 ms, 3862 cycles
- Task 5 at 179975.000 ms, 3313 cycles
- Task 0 at 179980.000 ms, 3253 cycles
- Task 0 at 179990.000 ms, 3189 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5256014 of them, the longest 14485 cycles (226.3 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4841396 |
| 512 to 1023 (8.0 to 16.0 µs) | 141399 |
| 1024 to 2047 (16.0 to 32.0 µs) | 94092 |
| 2048 to 4095 (32.0 to 64.0 µs) | 106966 |
| 4096 to 8191 (64.0 to 128.0 µs) | 66091 |
| 8192 to 16383 (128.0 to 256.0 µs) | 6070 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 53751 | 14485 / 226.3 |
| timer | 30979 | 14045 / 219.5 |
| job_end | 5137353 | 4529 / 70.8 |
| epilogue | 33931 | 4011 / 62.7 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 533428 | 18028 | 0.460 |
| 1 | 24617 | 6000 | 0.100 |
| 2 | 4804727 | 20996 | 5.280 |
| 3 | 1482708 | 5500 | 3.380 |
| 4 | 45193 | 9998 | 0.150 |
| 5 | 137669 | 7304 | 0.410 |
| 6 | 27676 | 1799 | 0.400 |
| none | 10940288 | 43388 | 9.310 |

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
loop:part0:sensor:hold=417
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=253
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
     Finished in 8.71s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:bf9c8606c1cc80ff08bfe0cf6dfe3734e74af9062f80a831a8800855816c27d4) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:d4faae210fcad55506e286ec2510e80a1398acb827111fa97f1d4dd36f9080fc is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 572 cycles (measured, rsk-hil release-timing record, 71997 runs, max 457 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2377 cycles (measured, rsk-hil release-timing record, 794 runs, max 1901 +25%, scenarios: nrf52840_cycles (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4105 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3284 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4170 cycles (measured, rsk-hil release-timing record, 1116 runs, max 3336 +25%, scenarios: nrf52840_cycles (2 s), conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 5015 cycles (measured, rsk-hil release-timing record, 49332 runs, max 4012 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 5755 cycles (measured, rsk-hil release-timing record, 10000 runs, max 4604 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1151 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 18107 cycles (measured, rsk-hil release-timing record, 5256014 runs, max 14485 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 14485 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 53751 (longest 14485), timer 30979 (longest 14045), job_end 5137353 (longest 4529), epilogue 33931 (longest 4011); the longest single Rust half 4012) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 5015 cycles (measured, rsk-hil release-timing record, 49332 runs, max 4012 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 8407 cycles (measured, rsk-hil release-timing record, 40495 runs, max 6725 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 285 cycles (measured, rsk-hil release-timing record, 4877646 runs, max 228 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 898 cycles (measured, rsk-hil release-timing record, 40 runs, max 718 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 1404 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2630 cycles (measured, rsk-hil release-timing record, 317 runs, max 2104 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 157 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2984 cycles (measured, rsk-hil release-timing record, 44552 runs, max 2387 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1175 cycles (measured, rsk-hil release-timing record, 90671 runs, max 940 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 96 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.035 | 10.000 | 0.775 | 18000 | within |
| `part0.echo` | A | 9.847 | 20.000 | 0.258 | 5999 | within |
| `part0.sensor` | A | 13.109 | 20.000 | 5.930 | 8999 | within |
| `part1.log` | C | 39.649 | 40.000 | 4.338 | 4500 | within |
| `part1.pulse` | C | 14.122 | 15.000 | 5.936 | 9999 | within |
| `part2.filter` | B | 16.964 | 25.000 | 6.351 | 7200 | within |
| `part2.sampler` | B | 79.255 | 100.000 | 4.718 | 1800 | within |

Verdict: **pass**
