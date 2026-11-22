# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:5ec8d193e80fbe2df2a805e545ed304ecad50f7bafb36c37e240b2b22f8ad155

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
| compare arms (HW-12, ASM-19) | pass | 288983 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (21 below 4 ticks, 56 below 8, 195 below 16); 6022 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7518 cycles (117.5 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 21067 cycles (329.2 µs) over 4301894 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 5550 cycles (86.7 µs) over 4376115 holds, closed by a timer entry; against the record's L_kernel 11000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10775 rises and 10001 response edges in the capture); latency 100 to 5020 µs, median 110 µs, 0 later than 15 ms; Δ_stamp observed 5692 cycles (88.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.6 % of the run, the Kernel 35.5 % over 1293666 entries; runs per Task [18031, 6001, 20995, 5047, 10260, 7201, 1843] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 29299 cycles (0.458 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 28160 cycles (0.440 ms at 100000 samples/s; 18031 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 334 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 203 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7518 / 117.5 | 3446 | 2^12:17489 2^13:510 |
| 2 | 8999 | 5639 / 88.1 | 3101 | 2^12:8951 2^13:48 |
| 3 | 4499 | 7518 / 117.5 | 3470 | 2^12:4376 2^13:123 |
| 5 | 7199 | 7518 / 117.5 | 3457 | 2^12:7007 2^13:192 |
| 6 | 1799 | 7518 / 117.5 | 3761 | 2^12:1746 2^13:53 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.797 | 0 | 0 | 0 |
| 1 | 5999 | 0.257 | 0 | 0 | 0 |
| 2 | 8999 | 6.013 | 0 | 0 | 0 |
| 3 | 4500 | 4.431 | 0 | 0 | 0 |
| 4 | 9999 | 5.042 | 10000 | 2193 | 1990 |
| 5 | 7200 | 6.470 | 0 | 0 | 0 |
| 6 | 1800 | 4.820 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 29299 / 0.458 (min 18226, mean 20661) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7274 / 0.114 (min 3523, mean 3642) | 0 above the Budget; 2^12:5994 2^13:5 |
| Task 2 entry | 8999 | 346858 / 5.420 (min 337838, mean 339556) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 222320 / 3.474 (min 205081, mean 213994) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 13678 / 0.214 (min 3598, mean 4108) | 0 above the Budget; 2^12:8865 2^13:1002 2^14:132 |
| Task 5 entry | 7200 | 25099 / 0.392 (min 10355, mean 14098) | 0 above the Budget; 2^14:5023 2^15:2177 |
| Task 6 entry | 1800 | 27837 / 0.435 (min 8593, mean 11717) | 0 above the Budget; 2^14:1766 2^15:34 |
| Task 0 on Resource 0 | 18000 | 7590 / 0.119 | |
| Task 0 on Resource 1 | 18000 | 4192 / 0.066 | |
| Task 2 on Resource 0 | 8999 | 328389 / 5.131 | |
| Task 2 on Resource 1 | 17998 | 4351 / 0.068 | |
| Task 5 on Resource 2 | 7200 | 4322 / 0.068 | |
| Task 6 on Resource 2 | 1800 | 7925 / 0.124 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3069 / 47.95 | 2861 |
| timer | 49461 | 3716 / 58.06 | 2784 |
| intake | 10000 | 2322 / 36.28 | 2131 |
| lock | 71997 | 488 / 7.62 | 486 |
| unlock | 71997 | 1807 / 28.23 | 1573 |
| job_end | 56497 | 1976 / 30.88 | 1684 |
| now | 3916589 | 248 / 3.88 | 245 |
| raise | 6000 | 1971 / 30.80 | 1909 |
| dma | 1855 | 737 / 11.52 | 548 |
| epilogue | 56497 | 1257 / 19.64 | 614 |
| view | 69344 | 327 / 5.11 | 294 |
| send | 44552 | 2246 / 35.09 | 2099 |
| recv | 90671 | 942 / 14.72 | 850 |

Entries that interrupted an executing Job: Task 0: 3055, Task 1: 5, Task 2: 28326, Task 3: 2636, Task 4: 1397, Task 5: 1068, Task 6: 1574

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 2 (timer), 5056 cycles; Task 1: 1 (timer), 2684 cycles; Task 2: 6 (dispatch, timer, intake), 16456 cycles; Task 3: 2 (timer), 5152 cycles; Task 4: 3 (dispatch, timer), 9686 cycles; Task 5: 2 (timer), 5252 cycles; Task 6: 1 (timer), 2760 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 4018 cycles
- Task 0 at 179940.000 ms, 3099 cycles
- Task 2 at 179945.000 ms, 3031 cycles
- Task 0 at 179950.000 ms, 3642 cycles
- Task 5 at 179950.000 ms, 3642 cycles
- Task 0 at 179960.000 ms, 5079 cycles
- Task 3 at 179960.000 ms, 5079 cycles
- Task 2 at 179965.000 ms, 3031 cycles
- Task 0 at 179970.000 ms, 3374 cycles
- Task 5 at 179975.000 ms, 3095 cycles
- Task 0 at 179980.000 ms, 3031 cycles
- Task 0 at 179990.000 ms, 2983 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 4301894 of them, the longest 21067 cycles (329.2 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 3982851 |
| 1024 to 2047 (16.0 to 32.0 µs) | 148263 |
| 2048 to 4095 (32.0 to 64.0 µs) | 104612 |
| 4096 to 8191 (64.0 to 128.0 µs) | 59307 |
| 8192 to 16383 (128.0 to 256.0 µs) | 6849 |
| 16384 to 32767 (256.0 to 512.0 µs) | 12 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 52869 | 21067 / 329.2 |
| timer | 31702 | 14026 / 219.2 |
| job_end | 4184436 | 4515 / 70.5 |
| epilogue | 32887 | 4149 / 64.8 |

The longest one's 7 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | job_end, an unwind follows, exception 56 pending | 0 | 2369 | 2369 / 37.0 | 0 / 0.0 |
| 2 | epilogue, exception 56 pending | 2555 | 3278 | 723 / 11.3 | 186 / 2.9 |
| 3 | dispatch, started a Job, exception 24 pending | 3558 | 6913 | 3355 / 52.4 | 280 / 4.4 |
| 4 | timer, exception 22 pending | 7194 | 10504 | 3310 / 51.7 | 281 / 4.4 |
| 5 | intake, exception 24 pending | 10802 | 13586 | 2784 / 43.5 | 298 / 4.7 |
| 6 | timer, exception 40 pending | 13884 | 17201 | 3317 / 51.8 | 298 / 4.7 |
| 7 | dispatch, started a Job | 17499 | 21067 | 3568 / 55.8 | 298 / 4.7 |

Rust halves 19426 cycles (303.5 µs), between them 1641 cycles (25.6 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 4376115 of them, the longest 5550 cycles (86.7 µs), closed by a timer entry:

The longest was one entry: timer, exception 36 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3658 / 57.2 |
| timer | 5550 / 86.7 |
| job_end | 2547 / 39.8 |
| epilogue | 4359 / 68.1 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 554270 | 18031 | 0.440 |
| 1 | 25109 | 6001 | 0.100 |
| 2 | 4827039 | 20995 | 5.300 |
| 3 | 1499673 | 5047 | 3.460 |
| 4 | 49162 | 10260 | 0.160 |
| 5 | 146212 | 7201 | 0.380 |
| 6 | 30091 | 1843 | 0.420 |
| none | 10864728 | 43643 | 9.430 |

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
loop:part0:sensor:hold=334
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=203
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
     Finished in 9.13s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:5ec8d193e80fbe2df2a805e545ed304ecad50f7bafb36c37e240b2b22f8ad155) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:5ec8d193e80fbe2df2a805e545ed304ecad50f7bafb36c37e240b2b22f8ad155 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 610 cycles (measured, rsk-hil release-timing record, 71997 runs, max 488 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2259 cycles (measured, rsk-hil release-timing record, 71997 runs, max 1807 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3837 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3069 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4042 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3233 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4645 cycles (measured, rsk-hil release-timing record, 49461 runs, max 3716 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 7115 cycles (measured, rsk-hil release-timing record, 10000 runs, max 5692 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1423 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 6938 cycles (measured, rsk-hil release-timing record, 4376115 runs, max 5550 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest hold of the run: the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler from its Rust half's beginning to its end with the unwind it runs before it returns (5550 cycles over 1 entries, closed by a timer entry; the longest single Rust half 3716; the longest run of entries back to back, each taken at an exception return and charged by the Analyzer as its own event's interference, 21067 cycles) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4645 cycles (measured, rsk-hil release-timing record, 49461 runs, max 3716 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_wake: 7115 cycles (measured, rsk-hil release-timing record, 5858 runs, max 5692 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from a Release_Source edge that arrived with no Job executing (the processor in the idle activity's WFI), latched by the time base through PPI, to the intake handler's timestamp: the wake-up from sleep, with the exception entry and the handler's read of the time base after it (an upper bound of the wake-up alone)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 9398 cycles (measured, rsk-hil release-timing record, 40495 runs, max 7518 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 310 cycles (measured, rsk-hil release-timing record, 3916589 runs, max 248 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 922 cycles (measured, rsk-hil release-timing record, 1855 runs, max 737 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 409 cycles (measured, rsk-hil release-timing record, 69344 runs, max 327 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2808 cycles (measured, rsk-hil release-timing record, 44552 runs, max 2246 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1178 cycles (measured, rsk-hil release-timing record, 90671 runs, max 942 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.226 | 10.000 | 0.797 | 18000 | within |
| `part0.echo` | A | 11.969 | 20.000 | 0.257 | 5999 | within |
| `part0.sensor` | A | 13.229 | 20.000 | 6.013 | 8999 | within |
| `part1.log` | C | 40.250 | 40.000 | 4.431 | 4500 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 14.315 | 15.000 | 5.042 | 9999 | within |
| `part2.filter` | B | 17.293 | 25.000 | 6.470 | 7200 | within |
| `part2.sampler` | B | 100.946 | 100.000 | 4.820 | 1800 | not schedulable: no WCRT to hold |

The plain configuration's analysis is not a PASS: the schedulability claim under measured timing does not hold for this set.

Verdict: **FAIL**
