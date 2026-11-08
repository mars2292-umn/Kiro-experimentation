# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.9 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7288 cycles (113.9 µs) against J_rel 12500 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16612 cycles (259.6 µs) over 6399454 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10658 rises and 10000 response edges in the capture); latency 90 to 5960 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 5636 cycles (88.1 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 43.4 % of the run, the Kernel 39.9 % over 723960 entries; runs per Task [18035, 6001, 20997, 5009, 10242, 7240, 1840] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 26463 cycles (0.413 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 25600 cycles (0.400 ms at 100000 samples/s; 18035 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:345 | 10_000_000 | 539 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:383 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:333 | 10_000_000 | 272 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7288 / 113.9 | 3682 | 2^12:17150 2^13:849 |
| 2 | 8999 | 5874 / 91.8 | 3274 | 2^12:8933 2^13:66 |
| 3 | 4499 | 7288 / 113.9 | 3649 | 2^12:4364 2^13:135 |
| 5 | 7199 | 7288 / 113.9 | 3685 | 2^12:6595 2^13:604 |
| 6 | 1799 | 7288 / 113.9 | 3957 | 2^12:1738 2^13:61 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.748 | 0 | 0 | 0 |
| 1 | 5999 | 0.245 | 0 | 0 | 0 |
| 2 | 8999 | 6.897 | 0 | 0 | 0 |
| 3 | 4500 | 4.186 | 0 | 0 | 0 |
| 4 | 9999 | 5.978 | 10000 | 2157 | 1997 |
| 5 | 7200 | 7.203 | 0 | 0 | 0 |
| 6 | 1800 | 4.425 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 26463 / 0.413 (min 14694, mean 18326) | 0 above the Budget; 2^14:5100 2^15:12900 |
| Task 1 entry | 5999 | 6888 / 0.108 (min 3191, mean 3279) | 0 above the Budget; 2^12:5998 2^13:1 |
| Task 2 entry | 8999 | 407270 / 6.364 (min 399462, mean 401235) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 213487 / 3.336 (min 200698, mean 206909) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 14490 / 0.226 (min 3251, mean 3388) | 0 above the Budget; 2^12:9868 2^13:9 2^14:122 |
| Task 5 entry | 7200 | 18006 / 0.281 (min 7663, mean 9307) | 0 above the Budget; 2^13:4859 2^14:2294 2^15:47 |
| Task 6 entry | 1800 | 19130 / 0.299 (min 7426, mean 9996) | 0 above the Budget; 2^13:584 2^14:1215 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7475 / 0.117 | |
| Task 0 on Resource 1 | 18000 | 4165 / 0.065 | |
| Task 2 on Resource 0 | 8999 | 389205 / 6.081 | |
| Task 2 on Resource 1 | 17998 | 4062 / 0.063 | |
| Task 5 on Resource 2 | 7200 | 5251 / 0.082 | |
| Task 6 on Resource 2 | 1800 | 5094 / 0.080 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2885 / 45.08 | 2735 |
| timer | 55403 | 3893 / 60.83 | 3005 |
| intake | 10000 | 2274 / 35.53 | 2116 |
| lock | 71997 | 469 / 7.33 | 467 |
| unlock | 71997 | 1830 / 28.59 | 1635 |
| job_end | 56497 | 2038 / 31.84 | 1784 |
| now | 6058270 | 240 / 3.75 | 238 |
| raise | 6000 | 2040 / 31.88 | 1962 |
| dma | 1808 | 705 / 11.02 | 501 |
| epilogue | 56497 | 1208 / 18.88 | 575 |
| view | 69326 | 329 / 5.14 | 294 |
| send | 33751 | 2289 / 35.77 | 2133 |
| recv | 64572 | 821 / 12.83 | 732 |

Entries that interrupted an executing Job: Task 0: 10728, Task 1: 1, Task 2: 29764, Task 3: 2536, Task 4: 384, Task 5: 195, Task 6: 1298

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3728 cycles
- Task 0 at 179940.000 ms, 3210 cycles
- Task 2 at 179945.000 ms, 3206 cycles
- Task 0 at 179950.000 ms, 4088 cycles
- Task 5 at 179950.000 ms, 4088 cycles
- Task 0 at 179960.000 ms, 3450 cycles
- Task 3 at 179960.000 ms, 3450 cycles
- Task 2 at 179965.000 ms, 3206 cycles
- Task 0 at 179970.000 ms, 3904 cycles
- Task 5 at 179975.000 ms, 3206 cycles
- Task 0 at 179980.000 ms, 3210 cycles
- Task 0 at 179990.000 ms, 3106 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 6399454 of them, the longest 16612 cycles (259.6 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 6044226 |
| 512 to 1023 (8.0 to 16.0 µs) | 103808 |
| 1024 to 2047 (16.0 to 32.0 µs) | 91640 |
| 2048 to 4095 (32.0 to 64.0 µs) | 90031 |
| 4096 to 8191 (64.0 to 128.0 µs) | 56594 |
| 8192 to 16383 (128.0 to 256.0 µs) | 13154 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 46442 | 16612 / 259.6 |
| timer | 37520 | 13471 / 210.5 |
| job_end | 6282599 | 5630 / 88.0 |
| epilogue | 32893 | 3906 / 61.0 |

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 493414 | 18035 | 0.400 |
| 1 | 23306 | 6001 | 0.090 |
| 2 | 5690137 | 20997 | 6.280 |
| 3 | 1450562 | 5009 | 3.320 |
| 4 | 40352 | 10242 | 0.150 |
| 5 | 94848 | 7240 | 0.260 |
| 6 | 25730 | 1840 | 0.230 |
| none | 10177941 | 43660 | 8.490 |

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
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=539
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=272
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
     Finished in 7.70s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:5212da0421e7a8df55bcc64d69369390bedddeaf88f44c7e314c585c2e9312f2) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:7107bc3df75cb35d0728869b8e643cc87dbf300fc64768a39262fe5bb5c6ea43 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 589 cycles (measured, rsk-hil release-timing record, 317 runs, max 471 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2313 cycles (measured, rsk-hil release-timing record, 317 runs, max 1850 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3607 cycles (measured, rsk-hil release-timing record, 56497 runs, max 2885 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4058 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3246 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4867 cycles (measured, rsk-hil release-timing record, 55403 runs, max 3893 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 7045 cycles (measured, rsk-hil release-timing record, 10000 runs, max 5636 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1409 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 20765 cycles (measured, rsk-hil release-timing record, 6399454 runs, max 16612 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 16612 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 46442 (longest 16612), timer 37520 (longest 13471), job_end 6282599 (longest 5630), epilogue 32893 (longest 3906); the longest single Rust half 3893) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 130 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1961, less Δ_entry of the plain cycle configuration, 2022, is 0, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 130 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 130 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1961, less Δ_entry of the plain cycle configuration, 2022, is 0, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 130 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4867 cycles (measured, rsk-hil release-timing record, 55403 runs, max 3893 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 9110 cycles (measured, rsk-hil release-timing record, 40495 runs, max 7288 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 300 cycles (measured, rsk-hil release-timing record, 6058270 runs, max 240 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 895 cycles (measured, rsk-hil release-timing record, 20 runs, max 716 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 702 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2528 cycles (measured, rsk-hil release-timing record, 149 runs, max 2022 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 149 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2862 cycles (measured, rsk-hil release-timing record, 33751 runs, max 2289 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1027 cycles (measured, rsk-hil release-timing record, 64572 runs, max 821 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.426 | 10.000 | 1.748 | 18000 | within |
| `part0.echo` | A | 11.913 | 20.000 | 0.245 | 5999 | within |
| `part0.sensor` | A | 13.380 | 20.000 | 6.897 | 8999 | within |
| `part1.log` | C | 39.785 | 40.000 | 4.186 | 4500 | within |
| `part1.pulse` | C | 14.393 | 15.000 | 5.978 | 9999 | within |
| `part2.filter` | B | 16.956 | 25.000 | 7.203 | 7200 | within |
| `part2.sampler` | B | 79.691 | 100.000 | 4.425 | 1800 | within |

Verdict: **pass**
