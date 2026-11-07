# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7254 cycles (113.3 µs) against J_rel 10000 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 19044 cycles (297.6 µs) over 6512270 spans, closed by a timer entry; against the record's L_kernel 25000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10644 rises and 10000 response edges in the capture); latency 90 to 6190 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 5504 cycles (86.0 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 41.9 % of the run, the Kernel 38.2 % over 493914 entries; runs per Task [18019, 6001, 20997, 4999, 10113, 7292, 1824] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20844 cycles (0.326 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 20480 cycles (0.320 ms at 100000 samples/s; 18019 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 558 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 280 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7254 / 113.3 | 3474 | 2^12:17661 2^13:338 |
| 2 | 8999 | 6182 / 96.6 | 3233 | 2^12:8926 2^13:73 |
| 3 | 4499 | 7254 / 113.3 | 3601 | 2^12:4369 2^13:130 |
| 5 | 7199 | 7254 / 113.3 | 3547 | 2^12:7091 2^13:108 |
| 6 | 1799 | 7254 / 113.3 | 3896 | 2^12:1741 2^13:58 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.605 | 0 | 0 | 0 |
| 1 | 5999 | 0.242 | 0 | 0 | 0 |
| 2 | 8999 | 6.840 | 0 | 0 | 0 |
| 3 | 4500 | 3.811 | 0 | 0 | 0 |
| 4 | 9999 | 6.212 | 10000 | 2162 | 2000 |
| 5 | 7200 | 7.133 | 0 | 0 | 0 |
| 6 | 1800 | 4.142 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20844 / 0.326 (min 9388, mean 12530) | 0 above the Budget; 2^14:17374 2^15:626 |
| Task 1 entry | 5999 | 6888 / 0.108 (min 3116, mean 3222) | 0 above the Budget; 2^12:5995 2^13:4 |
| Task 2 entry | 8999 | 407831 / 6.372 (min 399307, mean 401183) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 199567 / 3.118 (min 190090, mean 195528) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 13229 / 0.207 (min 3201, mean 3317) | 0 above the Budget; 2^12:9876 2^13:8 2^14:115 |
| Task 5 entry | 7200 | 14686 / 0.229 (min 6339, mean 6581) | 0 above the Budget; 2^13:6890 2^14:310 |
| Task 6 entry | 1800 | 18890 / 0.295 (min 7326, mean 9195) | 0 above the Budget; 2^13:906 2^14:893 2^15:1 |
| Task 0 on Resource 0 | 18000 | 4121 / 0.064 | |
| Task 0 on Resource 1 | 18000 | 4002 / 0.063 | |
| Task 2 on Resource 0 | 8999 | 388806 / 6.075 | |
| Task 2 on Resource 1 | 17998 | 3924 / 0.061 | |
| Task 5 on Resource 2 | 7200 | 5243 / 0.082 | |
| Task 6 on Resource 2 | 1800 | 5147 / 0.080 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2852 / 44.56 | 2699 |
| timer | 55210 | 3900 / 60.94 | 2964 |
| intake | 10000 | 2283 / 35.67 | 2117 |
| lock | 71997 | 480 / 7.50 | 478 |
| unlock | 71997 | 1804 / 28.19 | 1625 |
| job_end | 56497 | 2055 / 32.11 | 1792 |
| now | 6264631 | 236 / 3.69 | 234 |
| raise | 6000 | 2085 / 32.58 | 1998 |
| dma | 1808 | 734 / 11.47 | 501 |
| epilogue | 56497 | 1220 / 19.06 | 575 |
| view | 69221 | 330 / 5.16 | 295 |

Entries that interrupted an executing Job: Task 0: 10349, Task 1: 4, Task 2: 30204, Task 3: 2119, Task 4: 258, Task 5: 400, Task 6: 940

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3286 cycles
- Task 0 at 179940.000 ms, 3162 cycles
- Task 2 at 179945.000 ms, 3166 cycles
- Task 0 at 179950.000 ms, 3626 cycles
- Task 5 at 179950.000 ms, 3626 cycles
- Task 0 at 179960.000 ms, 3406 cycles
- Task 3 at 179960.000 ms, 3406 cycles
- Task 2 at 179965.000 ms, 3166 cycles
- Task 0 at 179970.000 ms, 3362 cycles
- Task 5 at 179975.000 ms, 3150 cycles
- Task 0 at 179980.000 ms, 3162 cycles
- Task 0 at 179990.000 ms, 3138 cycles

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 332169 | 18019 | 0.320 |
| 1 | 23700 | 6001 | 0.100 |
| 2 | 5689727 | 20997 | 6.240 |
| 3 | 1371041 | 4999 | 3.110 |
| 4 | 39807 | 10113 | 0.150 |
| 5 | 65232 | 7292 | 0.200 |
| 6 | 23535 | 1824 | 0.230 |
| none | 10451086 | 43765 | 8.610 |

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
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=558
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=280
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
     Finished in 7.73s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:87773056f8bf4206532f88c9b8315494f7078a5cedccbfb01520e48b9f710208) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:2a55da0b01f7d0e0e816de60a91f6edbc1c5f5fea6e15b8b6d9c486b35d79027 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 600 cycles (measured, rsk-hil release-timing record, 317 runs, max 480 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2267 cycles (measured, rsk-hil release-timing record, 317 runs, max 1813 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3565 cycles (measured, rsk-hil release-timing record, 56497 runs, max 2852 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4094 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3275 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4875 cycles (measured, rsk-hil release-timing record, 55210 runs, max 3900 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 6880 cycles (measured, rsk-hil release-timing record, 10000 runs, max 5504 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1376 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 23805 cycles (measured, rsk-hil release-timing record, 6512270 runs, max 19044 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 19044 cycles, closed by a timer entry; the longest single Rust half 3900) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 126 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1987, less Δ_entry of the plain cycle configuration, 1941, is 46, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 126 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 126 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1987, less Δ_entry of the plain cycle configuration, 1941, is 46, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 126 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4875 cycles (measured, rsk-hil release-timing record, 55210 runs, max 3900 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 9068 cycles (measured, rsk-hil release-timing record, 40495 runs, max 7254 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 295 cycles (measured, rsk-hil release-timing record, 55102 runs, max 236 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 919 cycles (measured, rsk-hil release-timing record, 20 runs, max 735 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 413 cycles (measured, rsk-hil release-timing record, 702 runs, max 330 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2427 cycles (measured, rsk-hil release-timing record, 151 runs, max 1941 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 151 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 96 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.237 | 10.000 | 1.605 | 18000 | within |
| `part0.echo` | A | 11.538 | 20.000 | 0.242 | 5999 | within |
| `part0.sensor` | A | 13.608 | 20.000 | 6.840 | 8999 | within |
| `part1.log` | C | 39.550 | 40.000 | 3.811 | 4500 | within |
| `part1.pulse` | C | 14.617 | 15.000 | 6.212 | 9999 | within |
| `part2.filter` | B | 17.040 | 25.000 | 7.133 | 7200 | within |
| `part2.sampler` | B | 79.177 | 100.000 | 4.142 | 1800 | within |

Verdict: **pass**
