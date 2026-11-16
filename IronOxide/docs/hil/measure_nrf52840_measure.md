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
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7034 cycles (109.9 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17082 cycles (266.9 µs) over 5265783 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10755 rises and 10001 response edges in the capture); latency 90 to 5900 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 4584 cycles (71.6 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.0 % of the run, the Kernel 33.5 % over 1052200 entries; runs per Task [18034, 6001, 20996, 5498, 10114, 7416, 1826] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 27514 cycles (0.430 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 26880 cycles (0.420 ms at 100000 samples/s; 18034 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 418 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 253 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6505 / 101.6 | 3504 | 2^12:17451 2^13:548 |
| 2 | 8999 | 7034 / 109.9 | 3254 | 2^12:8780 2^13:219 |
| 3 | 4499 | 6505 / 101.6 | 3548 | 2^12:4463 2^13:36 |
| 5 | 7199 | 7034 / 109.9 | 3583 | 2^12:6667 2^13:532 |
| 6 | 1799 | 6505 / 101.6 | 3842 | 2^12:1782 2^13:17 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.721 | 0 | 0 | 0 |
| 1 | 5999 | 0.250 | 0 | 0 | 0 |
| 2 | 8999 | 5.923 | 0 | 0 | 0 |
| 3 | 4500 | 4.230 | 0 | 0 | 0 |
| 4 | 9999 | 5.916 | 10000 | 2176 | 2012 |
| 5 | 7200 | 6.299 | 0 | 0 | 0 |
| 6 | 1800 | 4.592 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 27514 / 0.430 (min 16582, mean 18964) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 6950 / 0.109 (min 3342, mean 3435) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 343234 / 5.363 (min 336046, mean 337680) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 219993 / 3.437 (min 201281, mean 211070) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12728 / 0.199 (min 3426, mean 3895) | 0 above the Budget; 2^12:8855 2^13:1027 2^14:117 |
| Task 5 entry | 7200 | 24556 / 0.384 (min 9360, mean 12919) | 0 above the Budget; 2^14:5225 2^15:1975 |
| Task 6 entry | 1800 | 19958 / 0.312 (min 7734, mean 10360) | 0 above the Budget; 2^13:502 2^14:1297 2^15:1 |
| Task 0 on Resource 0 | 18000 | 4172 / 0.065 | |
| Task 0 on Resource 1 | 18000 | 4033 / 0.063 | |
| Task 2 on Resource 0 | 8999 | 325472 / 5.085 | |
| Task 2 on Resource 1 | 17998 | 3930 / 0.061 | |
| Task 5 on Resource 2 | 7200 | 5456 / 0.085 | |
| Task 6 on Resource 2 | 1800 | 5287 / 0.083 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56533 | 3104 / 48.50 | 2912 |
| timer | 49207 | 3863 / 60.36 | 2929 |
| intake | 10000 | 2288 / 35.75 | 2124 |
| lock | 71997 | 473 / 7.39 | 472 |
| unlock | 71997 | 1789 / 27.95 | 1595 |
| job_end | 56497 | 2012 / 31.44 | 1747 |
| now | 4882576 | 229 / 3.58 | 226 |
| raise | 6000 | 2068 / 32.31 | 1957 |
| dma | 1808 | 723 / 11.30 | 509 |
| epilogue | 56497 | 1246 / 19.47 | 620 |
| view | 69848 | 329 / 5.14 | 294 |
| send | 44552 | 2282 / 35.66 | 2121 |
| recv | 90671 | 899 / 14.05 | 807 |

Entries that interrupted an executing Job: Task 0: 3023, Task 1: 2, Task 2: 27872, Task 3: 3007, Task 4: 1262, Task 5: 1246, Task 6: 1343

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3862 cycles
- Task 0 at 179940.000 ms, 3149 cycles
- Task 2 at 179945.000 ms, 3149 cycles
- Task 0 at 179950.000 ms, 3658 cycles
- Task 5 at 179950.000 ms, 3658 cycles
- Task 0 at 179960.000 ms, 3449 cycles
- Task 3 at 179960.000 ms, 3449 cycles
- Task 2 at 179965.000 ms, 3149 cycles
- Task 0 at 179970.000 ms, 3378 cycles
- Task 5 at 179975.000 ms, 3153 cycles
- Task 0 at 179980.000 ms, 3209 cycles
- Task 0 at 179990.000 ms, 3077 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5265783 of them, the longest 17082 cycles (266.9 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4838488 |
| 512 to 1023 (8.0 to 16.0 µs) | 153906 |
| 1024 to 2047 (16.0 to 32.0 µs) | 105218 |
| 2048 to 4095 (32.0 to 64.0 µs) | 101378 |
| 4096 to 8191 (64.0 to 128.0 µs) | 60414 |
| 8192 to 16383 (128.0 to 256.0 µs) | 6378 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 53695 | 17082 / 266.9 |
| timer | 30067 | 15269 / 238.6 |
| job_end | 5147921 | 5345 / 83.5 |
| epilogue | 34100 | 7284 / 113.8 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 506599 | 18034 | 0.420 |
| 1 | 23682 | 6001 | 0.100 |
| 2 | 4796951 | 20996 | 5.240 |
| 3 | 1481429 | 5498 | 3.400 |
| 4 | 45411 | 10114 | 0.170 |
| 5 | 134631 | 7416 | 0.370 |
| 6 | 26241 | 1826 | 0.230 |
| none | 10981357 | 43140 | 9.480 |

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
loop:part0:sensor:hold=418
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=253
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
     Finished in 8.44s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:f9500dc167da40e9a984b5d08ef7a3c29e3073d3cdcf6afb5e23a384a445db03) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:95476a3e75d1025f107b5332e8908b4263fb251c92f55318c40566c2d78f0b02 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 592 cycles (measured, rsk-hil release-timing record, 71997 runs, max 473 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2237 cycles (measured, rsk-hil release-timing record, 71997 runs, max 1789 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3880 cycles (measured, rsk-hil release-timing record, 56533 runs, max 3104 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4073 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3258 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4829 cycles (measured, rsk-hil release-timing record, 49207 runs, max 3863 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 5730 cycles (measured, rsk-hil release-timing record, 10000 runs, max 4584 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1146 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 21353 cycles (measured, rsk-hil release-timing record, 5265783 runs, max 17082 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 17082 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 53695 (longest 17082), timer 30067 (longest 15269), job_end 5147921 (longest 5345), epilogue 34100 (longest 7284); the longest single Rust half 3863) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 98 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2175, less Δ_entry of the plain cycle configuration, 2121, is 54, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 98 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 98 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2175, less Δ_entry of the plain cycle configuration, 2121, is 54, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 98 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4829 cycles (measured, rsk-hil release-timing record, 49207 runs, max 3863 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 8793 cycles (measured, rsk-hil release-timing record, 40495 runs, max 7034 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 287 cycles (measured, rsk-hil release-timing record, 4882576 runs, max 229 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 904 cycles (measured, rsk-hil release-timing record, 1808 runs, max 723 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 565 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2652 cycles (measured, rsk-hil release-timing record, 118 runs, max 2121 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 118 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2853 cycles (measured, rsk-hil release-timing record, 44552 runs, max 2282 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1124 cycles (measured, rsk-hil release-timing record, 90671 runs, max 899 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 95 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 8.978 | 10.000 | 0.721 | 18000 | within |
| `part0.echo` | A | 9.775 | 20.000 | 0.250 | 5999 | within |
| `part0.sensor` | A | 13.043 | 20.000 | 5.923 | 8999 | within |
| `part1.log` | C | 39.258 | 40.000 | 4.230 | 4500 | within |
| `part1.pulse` | C | 14.041 | 15.000 | 5.916 | 9999 | within |
| `part2.filter` | B | 16.830 | 25.000 | 6.299 | 7200 | within |
| `part2.sampler` | B | 78.502 | 100.000 | 4.592 | 1800 | within |

Verdict: **pass**
