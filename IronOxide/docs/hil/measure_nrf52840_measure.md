# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 194 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7458 cycles (116.5 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 15519 cycles (242.5 µs) over 5297788 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10755 rises and 10001 response edges in the capture); latency 90 to 5430 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 3292 cycles (51.4 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.0 % of the run, the Kernel 33.6 % over 1022879 entries; runs per Task [18035, 6000, 20997, 5278, 10118, 7200, 1801] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 27557 cycles (0.431 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 26880 cycles (0.420 ms at 100000 samples/s; 18035 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 420 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 255 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7458 / 116.5 | 3528 | 2^12:17604 2^13:395 |
| 2 | 8999 | 5253 / 82.1 | 3262 | 2^12:8970 2^13:29 |
| 3 | 4499 | 7458 / 116.5 | 3626 | 2^12:4401 2^13:98 |
| 5 | 7199 | 7458 / 116.5 | 3590 | 2^12:6933 2^13:266 |
| 6 | 1799 | 7458 / 116.5 | 3924 | 2^12:1754 2^13:45 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.695 | 0 | 0 | 0 |
| 1 | 5999 | 0.247 | 0 | 0 | 0 |
| 2 | 8999 | 5.909 | 0 | 0 | 0 |
| 3 | 4500 | 4.245 | 0 | 0 | 0 |
| 4 | 9999 | 5.440 | 10000 | 2227 | 2060 |
| 5 | 7200 | 6.307 | 0 | 0 | 0 |
| 6 | 1800 | 4.608 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 27557 / 0.431 (min 16661, mean 19008) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7011 / 0.110 (min 3291, mean 3387) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8999 | 342960 / 5.359 (min 336120, mean 337882) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 218709 / 3.417 (min 202325, mean 211541) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 14957 / 0.234 (min 3377, mean 3858) | 0 above the Budget; 2^12:8878 2^13:1000 2^14:121 |
| Task 5 entry | 7200 | 25229 / 0.394 (min 9345, mean 12840) | 0 above the Budget; 2^14:5218 2^15:1982 |
| Task 6 entry | 1800 | 19973 / 0.312 (min 7733, mean 10238) | 0 above the Budget; 2^13:579 2^14:1220 2^15:1 |
| Task 0 on Resource 0 | 18000 | 4226 / 0.066 | |
| Task 0 on Resource 1 | 18000 | 4063 / 0.063 | |
| Task 2 on Resource 0 | 8999 | 325395 / 5.084 | |
| Task 2 on Resource 1 | 17998 | 4128 / 0.065 | |
| Task 5 on Resource 2 | 7200 | 7741 / 0.121 | |
| Task 6 on Resource 2 | 1800 | 4081 / 0.064 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3056 / 47.75 | 2861 |
| timer | 49036 | 3907 / 61.05 | 2976 |
| intake | 10000 | 2338 / 36.53 | 2170 |
| lock | 71997 | 472 / 7.38 | 470 |
| unlock | 71997 | 1816 / 28.38 | 1638 |
| job_end | 56497 | 2061 / 32.20 | 1802 |
| now | 4913578 | 226 / 3.53 | 223 |
| raise | 6000 | 2123 / 33.17 | 2008 |
| dma | 1808 | 721 / 11.27 | 507 |
| epilogue | 56497 | 1261 / 19.70 | 615 |
| view | 69389 | 329 / 5.14 | 294 |
| send | 44552 | 2307 / 36.05 | 2156 |
| recv | 90671 | 901 / 14.08 | 805 |

Entries that interrupted an executing Job: Task 0: 2716, Task 1: 3, Task 2: 28193, Task 3: 2800, Task 4: 1271, Task 5: 860, Task 6: 1248

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3914 cycles
- Task 0 at 179940.000 ms, 3197 cycles
- Task 2 at 179945.000 ms, 3201 cycles
- Task 0 at 179950.000 ms, 3626 cycles
- Task 5 at 179950.000 ms, 3626 cycles
- Task 0 at 179960.000 ms, 3441 cycles
- Task 3 at 179960.000 ms, 3441 cycles
- Task 2 at 179965.000 ms, 3201 cycles
- Task 0 at 179970.000 ms, 3354 cycles
- Task 5 at 179975.000 ms, 3197 cycles
- Task 0 at 179980.000 ms, 3197 cycles
- Task 0 at 179990.000 ms, 3181 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5297788 of them, the longest 15519 cycles (242.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4904685 |
| 512 to 1023 (8.0 to 16.0 µs) | 119453 |
| 1024 to 2047 (16.0 to 32.0 µs) | 102033 |
| 2048 to 4095 (32.0 to 64.0 µs) | 104792 |
| 4096 to 8191 (64.0 to 128.0 µs) | 61370 |
| 8192 to 16383 (128.0 to 256.0 µs) | 5455 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 54117 | 15519 / 242.5 |
| timer | 30962 | 13534 / 211.5 |
| job_end | 5179498 | 4617 / 72.1 |
| epilogue | 33211 | 7472 / 116.8 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 510368 | 18035 | 0.420 |
| 1 | 24144 | 6000 | 0.100 |
| 2 | 4800607 | 20997 | 5.250 |
| 3 | 1484223 | 5278 | 3.400 |
| 4 | 46392 | 10118 | 0.150 |
| 5 | 133723 | 7200 | 0.380 |
| 6 | 26026 | 1801 | 0.220 |
| none | 10970803 | 43597 | 9.480 |

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
loop:part0:sensor:hold=420
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=255
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
     Finished in 8.45s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:12c8512c2f775c51b7bcc27e169a27a96f09c013a883dfe9640bc9df0067cac3) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:e1faf204ae173e8765f2f5c1ee1724400a4644cd28482c4dc9f98838e5420e93 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 590 cycles (measured, rsk-hil release-timing record, 143994 runs, max 472 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2292 cycles (measured, rsk-hil release-timing record, 397 runs, max 1833 +25%, scenarios: nrf52840_cycles (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3825 cycles (measured, rsk-hil release-timing record, 558 runs, max 3060 +25%, scenarios: nrf52840_cycles (2 s), conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4153 cycles (measured, rsk-hil release-timing record, 112995 runs, max 3322 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4949 cycles (measured, rsk-hil release-timing record, 98078 runs, max 3959 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 5605 cycles (measured, rsk-hil release-timing record, 20000 runs, max 4484 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1121 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 21114 cycles (measured, rsk-hil release-timing record, 10589322 runs, max 16891 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 16891 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 53698 (longest 16891), timer 30160 (longest 14890), job_end 5174319 (longest 3999), epilogue 33357 (longest 3952); the longest single Rust half 3959) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4949 cycles (measured, rsk-hil release-timing record, 98078 runs, max 3959 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 9323 cycles (measured, rsk-hil release-timing record, 80990 runs, max 7458 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 287 cycles (measured, rsk-hil release-timing record, 49082 runs, max 229 +25%, scenarios: nrf52840_cycles (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 902 cycles (measured, rsk-hil release-timing record, 3616 runs, max 721 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 753 runs, max 329 +25%, scenarios: nrf52840_cycles (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2594 cycles (measured, rsk-hil release-timing record, 152 runs, max 2075 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 152 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2900 cycles (measured, rsk-hil release-timing record, 89104 runs, max 2320 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1127 cycles (measured, rsk-hil release-timing record, 181342 runs, max 901 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 96 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.025 | 10.000 | 0.695 | 18000 | within |
| `part0.echo` | A | 9.833 | 20.000 | 0.247 | 5999 | within |
| `part0.sensor` | A | 13.129 | 20.000 | 5.909 | 8999 | within |
| `part1.log` | C | 39.612 | 40.000 | 4.245 | 4500 | within |
| `part1.pulse` | C | 14.137 | 15.000 | 5.440 | 9999 | within |
| `part2.filter` | B | 16.961 | 25.000 | 6.307 | 7200 | within |
| `part2.sampler` | B | 79.209 | 100.000 | 4.608 | 1800 | within |

Verdict: **pass**
