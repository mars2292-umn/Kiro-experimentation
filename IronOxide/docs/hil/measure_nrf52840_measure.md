# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 193 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5646 cycles (88.2 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16993 cycles (265.5 µs) over 5169307 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10698 rises and 10000 response edges in the capture); latency 100 to 4830 µs, median 110 µs, 0 later than 15 ms; Δ_stamp observed 5268 cycles (82.3 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.1 % of the run, the Kernel 33.8 % over 1144591 entries; runs per Task [18035, 5999, 20995, 5002, 9998, 7199, 1839] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 29890 cycles (0.467 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 28800 cycles (0.450 ms at 100000 samples/s; 18035 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 409 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 249 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5394 / 84.3 | 3689 | 2^12:16645 2^13:1354 |
| 2 | 8999 | 5646 / 88.2 | 3267 | 2^12:8997 2^13:2 |
| 3 | 4499 | 5394 / 84.3 | 3601 | 2^12:4397 2^13:102 |
| 5 | 7199 | 4307 / 67.3 | 3690 | 2^12:5850 2^13:1349 |
| 6 | 1799 | 4106 / 64.2 | 3916 | 2^12:1699 2^13:100 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.717 | 0 | 0 | 0 |
| 1 | 5999 | 0.252 | 0 | 0 | 0 |
| 2 | 8999 | 5.954 | 0 | 0 | 0 |
| 3 | 4500 | 4.339 | 0 | 0 | 0 |
| 4 | 9999 | 4.845 | 10000 | 2260 | 2020 |
| 5 | 7200 | 6.370 | 0 | 0 | 0 |
| 6 | 1800 | 4.611 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 29890 / 0.467 (min 16849, mean 19349) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7281 / 0.114 (min 3502, mean 3645) | 0 above the Budget; 2^12:5934 2^13:65 |
| Task 2 entry | 8999 | 344250 / 5.379 (min 336302, mean 338134) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 218889 / 3.420 (min 202997, mean 211757) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 7326 / 0.114 (min 3558, mean 3962) | 0 above the Budget; 2^12:8994 2^13:1005 |
| Task 5 entry | 7200 | 21752 / 0.340 (min 9548, mean 13067) | 0 above the Budget; 2^14:5298 2^15:1902 |
| Task 6 entry | 1800 | 20246 / 0.316 (min 7902, mean 10788) | 0 above the Budget; 2^13:420 2^14:1370 2^15:10 |
| Task 0 on Resource 0 | 18000 | 4169 / 0.065 | |
| Task 0 on Resource 1 | 18000 | 4139 / 0.065 | |
| Task 2 on Resource 0 | 8999 | 325557 / 5.087 | |
| Task 2 on Resource 1 | 17998 | 4151 / 0.065 | |
| Task 5 on Resource 2 | 7200 | 7717 / 0.121 | |
| Task 6 on Resource 2 | 1800 | 7558 / 0.118 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3276 / 51.19 | 3052 |
| timer | 49392 | 3857 / 60.27 | 2997 |
| intake | 10000 | 2374 / 37.09 | 2134 |
| lock | 71997 | 476 / 7.44 | 475 |
| unlock | 71997 | 1881 / 29.39 | 1622 |
| job_end | 56497 | 2058 / 32.16 | 1739 |
| now | 4790897 | 234 / 3.66 | 233 |
| raise | 6000 | 2096 / 32.75 | 1969 |
| dma | 1808 | 727 / 11.36 | 505 |
| epilogue | 56497 | 1255 / 19.61 | 606 |
| view | 69036 | 329 / 5.14 | 294 |
| send | 44552 | 2297 / 35.89 | 2128 |
| recv | 90671 | 902 / 14.09 | 811 |

Entries that interrupted an executing Job: Task 0: 3602, Task 1: 65, Task 2: 28309, Task 3: 2523, Task 4: 1005, Task 5: 928, Task 6: 1448

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3927 cycles
- Task 0 at 179940.000 ms, 3266 cycles
- Task 2 at 179945.000 ms, 3218 cycles
- Task 0 at 179950.000 ms, 4307 cycles
- Task 5 at 179950.000 ms, 4307 cycles
- Task 0 at 179960.000 ms, 3498 cycles
- Task 3 at 179960.000 ms, 3498 cycles
- Task 2 at 179965.000 ms, 3218 cycles
- Task 0 at 179970.000 ms, 4023 cycles
- Task 5 at 179975.000 ms, 3222 cycles
- Task 0 at 179980.000 ms, 3214 cycles
- Task 0 at 179990.000 ms, 3134 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5169307 of them, the longest 16993 cycles (265.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4776811 |
| 512 to 1023 (8.0 to 16.0 µs) | 118883 |
| 1024 to 2047 (16.0 to 32.0 µs) | 104139 |
| 2048 to 4095 (32.0 to 64.0 µs) | 98340 |
| 4096 to 8191 (64.0 to 128.0 µs) | 64872 |
| 8192 to 16383 (128.0 to 256.0 µs) | 6250 |
| 16384 to 32767 (256.0 to 512.0 µs) | 12 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 53289 | 16993 / 265.5 |
| timer | 32147 | 13530 / 211.4 |
| job_end | 5050948 | 5663 / 88.5 |
| epilogue | 32923 | 3928 / 61.4 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 513405 | 18035 | 0.450 |
| 1 | 24024 | 5999 | 0.100 |
| 2 | 4804193 | 20995 | 5.250 |
| 3 | 1482612 | 5002 | 3.400 |
| 4 | 43473 | 9998 | 0.100 |
| 5 | 133216 | 7199 | 0.330 |
| 6 | 27080 | 1839 | 0.240 |
| none | 10968297 | 43951 | 9.470 |

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
loop:part1:log:busy=249
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
     Finished in 8.04s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:d44020a9efabad6848227822ea8eb9e3e5cf438ccbaef7d39619e3c07959ebf1) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:d44020a9efabad6848227822ea8eb9e3e5cf438ccbaef7d39619e3c07959ebf1 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 595 cycles (measured, rsk-hil release-timing record, 71997 runs, max 476 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2352 cycles (measured, rsk-hil release-timing record, 71997 runs, max 1881 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4095 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3276 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4142 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3313 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4822 cycles (measured, rsk-hil release-timing record, 49392 runs, max 3857 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 6585 cycles (measured, rsk-hil release-timing record, 10000 runs, max 5268 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1317 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 21242 cycles (measured, rsk-hil release-timing record, 5169307 runs, max 16993 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 16993 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 53289 (longest 16993), timer 32147 (longest 13530), job_end 5050948 (longest 5663), epilogue 32923 (longest 3928); the longest single Rust half 3857) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4822 cycles (measured, rsk-hil release-timing record, 49392 runs, max 3857 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 7058 cycles (measured, rsk-hil release-timing record, 40495 runs, max 5646 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 293 cycles (measured, rsk-hil release-timing record, 4790897 runs, max 234 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 909 cycles (measured, rsk-hil release-timing record, 1808 runs, max 727 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 69036 runs, max 329 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2872 cycles (measured, rsk-hil release-timing record, 44552 runs, max 2297 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1128 cycles (measured, rsk-hil release-timing record, 90671 runs, max 902 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 95 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 8.931 | 10.000 | 0.717 | 18000 | within |
| `part0.echo` | A | 9.717 | 20.000 | 0.252 | 5999 | within |
| `part0.sensor` | A | 12.940 | 20.000 | 5.954 | 8999 | within |
| `part1.log` | C | 39.067 | 40.000 | 4.339 | 4500 | within |
| `part1.pulse` | C | 13.925 | 15.000 | 4.845 | 9999 | within |
| `part2.filter` | B | 16.680 | 25.000 | 6.370 | 7200 | within |
| `part2.sampler` | B | 39.873 | 100.000 | 4.611 | 1800 | within |

Verdict: **pass**
