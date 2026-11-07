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
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6916 cycles (108.1 µs) against J_rel 10000 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16202 cycles (253.2 µs) over 6415349 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10652 rises and 10000 response edges in the capture); latency 90 to 5870 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 5704 cycles (89.1 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 42.0 % of the run, the Kernel 38.4 % over 594053 entries; runs per Task [18036, 6000, 20997, 5089, 10001, 7236, 1800] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20366 cycles (0.318 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18036 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 549 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 277 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6916 / 108.1 | 3448 | 2^12:17832 2^13:167 |
| 2 | 8999 | 6107 / 95.4 | 3247 | 2^12:8943 2^13:56 |
| 3 | 4499 | 6916 / 108.1 | 3568 | 2^12:4461 2^13:38 |
| 5 | 7199 | 6512 / 101.8 | 3535 | 2^12:7141 2^13:58 |
| 6 | 1799 | 6503 / 101.6 | 3856 | 2^12:1783 2^13:16 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.596 | 0 | 0 | 0 |
| 1 | 5999 | 0.245 | 0 | 0 | 0 |
| 2 | 8999 | 6.820 | 0 | 0 | 0 |
| 3 | 4500 | 3.837 | 0 | 0 | 0 |
| 4 | 9999 | 5.899 | 10000 | 2208 | 2040 |
| 5 | 7200 | 7.067 | 0 | 0 | 0 |
| 6 | 1800 | 4.154 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20366 / 0.318 (min 9482, mean 12804) | 0 above the Budget; 2^14:17040 2^15:960 |
| Task 1 entry | 5999 | 6951 / 0.109 (min 3147, mean 3253) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 407134 / 6.361 (min 399438, mean 401648) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 201058 / 3.142 (min 190066, mean 195726) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 9728 / 0.152 (min 3232, mean 3278) | 0 above the Budget; 2^12:9983 2^13:13 2^14:3 |
| Task 5 entry | 7200 | 11456 / 0.179 (min 6398, mean 6533) | 0 above the Budget; 2^13:7097 2^14:103 |
| Task 6 entry | 1800 | 19019 / 0.297 (min 7387, mean 9072) | 0 above the Budget; 2^13:968 2^14:831 2^15:1 |
| Task 0 on Resource 0 | 18000 | 6988 / 0.109 | |
| Task 0 on Resource 1 | 18000 | 4145 / 0.065 | |
| Task 2 on Resource 0 | 8999 | 389485 / 6.086 | |
| Task 2 on Resource 1 | 17998 | 4057 / 0.063 | |
| Task 5 on Resource 2 | 7200 | 5330 / 0.083 | |
| Task 6 on Resource 2 | 1800 | 3960 / 0.062 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2890 / 45.16 | 2740 |
| timer | 55169 | 3791 / 59.23 | 2988 |
| intake | 10000 | 2335 / 36.48 | 2166 |
| lock | 71997 | 465 / 7.27 | 463 |
| unlock | 71997 | 1843 / 28.80 | 1651 |
| job_end | 56497 | 2068 / 32.31 | 1799 |
| now | 6167447 | 240 / 3.75 | 240 |
| raise | 6000 | 2110 / 32.97 | 2017 |
| dma | 1808 | 733 / 11.45 | 503 |
| epilogue | 56497 | 1225 / 19.14 | 582 |
| view | 69125 | 330 / 5.16 | 295 |

Entries that interrupted an executing Job: Task 0: 11268, Task 1: 2, Task 2: 30353, Task 3: 2251, Task 4: 19, Task 5: 140, Task 6: 833

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3900 cycles
- Task 0 at 179940.000 ms, 3183 cycles
- Task 2 at 179945.000 ms, 3187 cycles
- Task 0 at 179950.000 ms, 3616 cycles
- Task 5 at 179950.000 ms, 3616 cycles
- Task 0 at 179960.000 ms, 3411 cycles
- Task 3 at 179960.000 ms, 3411 cycles
- Task 2 at 179965.000 ms, 3187 cycles
- Task 0 at 179970.000 ms, 3348 cycles
- Task 5 at 179975.000 ms, 3179 cycles
- Task 0 at 179980.000 ms, 3183 cycles
- Task 0 at 179990.000 ms, 3163 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 6415349 of them, the longest 16202 cycles (253.2 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 6158884 |
| 512 to 1023 (8.0 to 16.0 µs) | 73361 |
| 1024 to 2047 (16.0 to 32.0 µs) | 55582 |
| 2048 to 4095 (32.0 to 64.0 µs) | 60836 |
| 4096 to 8191 (64.0 to 128.0 µs) | 55520 |
| 8192 to 16383 (128.0 to 256.0 µs) | 11166 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 47235 | 16202 / 253.2 |
| timer | 36907 | 13172 / 205.8 |
| job_end | 6299493 | 5495 / 85.9 |
| epilogue | 31714 | 3896 / 60.9 |

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 338400 | 18036 | 0.300 |
| 1 | 23387 | 6000 | 0.100 |
| 2 | 5695712 | 20997 | 6.290 |
| 3 | 1372438 | 5089 | 3.110 |
| 4 | 37601 | 10001 | 0.130 |
| 5 | 63697 | 7236 | 0.160 |
| 6 | 22930 | 1800 | 0.210 |
| none | 10442138 | 43861 | 8.650 |

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
loop:part0:sensor:hold=549
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=277
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
     Finished in 7.75s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:99eaca9e9d14c588f18cd8e1cc2977c4442ed65ffc30d888fb4f6f62fdcdb2ca) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:99eaca9e9d14c588f18cd8e1cc2977c4442ed65ffc30d888fb4f6f62fdcdb2ca is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 582 cycles (measured, rsk-hil release-timing record, 71997 runs, max 465 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2304 cycles (measured, rsk-hil release-timing record, 71997 runs, max 1843 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3613 cycles (measured, rsk-hil release-timing record, 56497 runs, max 2890 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4117 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3293 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4739 cycles (measured, rsk-hil release-timing record, 55169 runs, max 3791 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 7130 cycles (measured, rsk-hil release-timing record, 10000 runs, max 5704 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1426 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 20253 cycles (measured, rsk-hil release-timing record, 6415349 runs, max 16202 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 16202 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 47235 (longest 16202), timer 36907 (longest 13172), job_end 6299493 (longest 5495), epilogue 31714 (longest 3896); the longest single Rust half 3791) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4739 cycles (measured, rsk-hil release-timing record, 55169 runs, max 3791 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 8645 cycles (measured, rsk-hil release-timing record, 40495 runs, max 6916 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 300 cycles (measured, rsk-hil release-timing record, 6167447 runs, max 240 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 917 cycles (measured, rsk-hil release-timing record, 1808 runs, max 733 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 413 cycles (measured, rsk-hil release-timing record, 69125 runs, max 330 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 95 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.213 | 10.000 | 1.596 | 18000 | within |
| `part0.echo` | A | 9.995 | 20.000 | 0.245 | 5999 | within |
| `part0.sensor` | A | 13.427 | 20.000 | 6.820 | 8999 | within |
| `part1.log` | C | 39.122 | 40.000 | 3.837 | 4500 | within |
| `part1.pulse` | C | 14.410 | 15.000 | 5.899 | 9999 | within |
| `part2.filter` | B | 16.770 | 25.000 | 7.067 | 7200 | within |
| `part2.sampler` | B | 39.855 | 100.000 | 4.154 | 1800 | within |

Verdict: **pass**
