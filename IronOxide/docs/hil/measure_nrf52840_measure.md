# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7083 cycles (110.7 µs) against J_rel 10000 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10625 rises and 10000 response edges in the capture); latency 80 to 5720 µs, median 90 µs, 0 later than 15 ms; Δ_stamp observed 5416 cycles (84.6 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 41.6 % of the run, the Kernel 35.4 % over 1158100 entries; runs per Task [18080, 6001, 20999, 4999, 10208, 7200, 1799] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 19841 cycles (0.310 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18080 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 688 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 347 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7083 / 110.7 | 3382 | 2^12:17774 2^13:225 |
| 2 | 8999 | 5928 / 92.6 | 3109 | 2^12:8938 2^13:61 |
| 3 | 4499 | 7083 / 110.7 | 3479 | 2^12:4419 2^13:80 |
| 5 | 7199 | 7083 / 110.7 | 3443 | 2^12:7122 2^13:77 |
| 6 | 1799 | 7083 / 110.7 | 3786 | 2^12:1763 2^13:36 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.565 | 0 | 0 | 0 |
| 1 | 5999 | 0.248 | 0 | 0 | 0 |
| 2 | 8999 | 6.755 | 0 | 0 | 0 |
| 3 | 4500 | 3.786 | 0 | 0 | 0 |
| 4 | 9999 | 5.737 | 10000 | 2084 | 1899 |
| 5 | 7200 | 6.990 | 0 | 0 | 0 |
| 6 | 1800 | 4.066 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19841 / 0.310 (min 8614, mean 11790) | 0 above the Budget; 2^14:17746 2^15:254 |
| Task 1 entry | 5999 | 6632 / 0.104 (min 2973, mean 3143) | 0 above the Budget; 2^12:5889 2^13:110 |
| Task 2 entry | 8999 | 405144 / 6.330 (min 397872, mean 399424) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 199199 / 3.112 (min 190128, mean 195172) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 11914 / 0.186 (min 3057, mean 3175) | 0 above the Budget; 2^12:9886 2^13:8 2^14:105 |
| Task 5 entry | 7200 | 9586 / 0.150 (min 5886, mean 5997) | 0 above the Budget; 2^13:7136 2^14:64 |
| Task 6 entry | 1800 | 17003 / 0.266 (min 6735, mean 8367) | 0 above the Budget; 2^13:957 2^14:842 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7303 / 0.114 | |
| Task 0 on Resource 1 | 18000 | 3800 / 0.059 | |
| Task 2 on Resource 0 | 8999 | 388505 / 6.070 | |
| Task 2 on Resource 1 | 17998 | 3705 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 3681 / 0.058 | |
| Task 6 on Resource 2 | 1800 | 3700 / 0.058 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2819 / 44.05 | 2646 |
| timer | 55214 | 3819 / 59.67 | 2898 |
| intake | 10000 | 2210 / 34.53 | 2024 |
| lock | 71997 | 464 / 7.25 | 462 |
| unlock | 71997 | 1772 / 27.69 | 1587 |
| job_end | 56497 | 1993 / 31.14 | 1727 |
| now | 7727441 | 242 / 3.78 | 241 |
| raise | 6000 | 2065 / 32.27 | 1949 |
| dma | 1808 | 712 / 11.12 | 498 |
| epilogue | 56497 | 1221 / 19.08 | 584 |
| view | 69205 | 328 / 5.12 | 292 |

Entries that interrupted an executing Job: Task 0: 11268, Task 1: 110, Task 2: 29725, Task 3: 2168, Task 4: 323, Task 5: 64, Task 6: 844

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3403 cycles
- Task 0 at 179940.000 ms, 3044 cycles
- Task 2 at 179945.000 ms, 3044 cycles
- Task 0 at 179950.000 ms, 3623 cycles
- Task 5 at 179950.000 ms, 3623 cycles
- Task 0 at 179960.000 ms, 3292 cycles
- Task 3 at 179960.000 ms, 3292 cycles
- Task 2 at 179965.000 ms, 3044 cycles
- Task 0 at 179970.000 ms, 3439 cycles
- Task 5 at 179975.000 ms, 3040 cycles
- Task 0 at 179980.000 ms, 3044 cycles
- Task 0 at 179990.000 ms, 2928 cycles

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 310737 | 18080 | 0.300 |
| 1 | 22565 | 6001 | 0.090 |
| 2 | 5663220 | 20999 | 6.220 |
| 3 | 1368343 | 4999 | 3.110 |
| 4 | 37711 | 10208 | 0.140 |
| 5 | 58004 | 7200 | 0.140 |
| 6 | 21007 | 1799 | 0.190 |
| none | 10514714 | 43781 | 8.620 |

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
loop:part0:sensor:hold=688
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=347
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
     Finished in 7.34s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:c7cd2449a365484f9d71434a19a1baab50eb2a583303f2dd1aac5afdf91aabd2) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:43bf3c0e2a0625c9699cbb205da250beb2b25c9eca7a28962c2dabeedca0da0d is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 580 cycles (measured, rsk-hil release-timing record, 634 runs, max 464 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2225 cycles (measured, rsk-hil release-timing record, 634 runs, max 1780 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3524 cycles (measured, rsk-hil release-timing record, 56497 runs, max 2819 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4018 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3214 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4774 cycles (measured, rsk-hil release-timing record, 55214 runs, max 3819 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 6770 cycles (measured, rsk-hil release-timing record, 10000 runs, max 5416 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1354 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 6578 cycles (measured, rsk-hil release-timing record, 215718 runs, max 5262 +25%, scenarios: nrf52840_cycles (2 s), conditions: the longest Kernel entry of the run (every entry masks preemption, DD-01): the largest Rust half of any kind plus the stub's measured 1800 cycles outside the charge points (Δ_entry) (task 19.3; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 117 cycles (measured, rsk-hil release-timing record, 256 runs, max 93 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1874, less Δ_entry of the plain cycle configuration, 1800, is 74, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 117 cycles (measured, rsk-hil release-timing record, 256 runs, max 93 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1874, less Δ_entry of the plain cycle configuration, 1800, is 74, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4774 cycles (measured, rsk-hil release-timing record, 55214 runs, max 3819 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 8854 cycles (measured, rsk-hil release-timing record, 40495 runs, max 7083 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 303 cycles (measured, rsk-hil release-timing record, 135838 runs, max 242 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 890 cycles (measured, rsk-hil release-timing record, 40 runs, max 712 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 410 cycles (measured, rsk-hil release-timing record, 1401 runs, max 328 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2250 cycles (measured, rsk-hil release-timing record, 440 runs, max 1800 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 144 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 96 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.218 | 10.000 | 1.565 | 18000 | within |
| `part0.echo` | A | 11.500 | 20.000 | 0.248 | 5999 | within |
| `part0.sensor` | A | 13.297 | 20.000 | 6.755 | 8999 | within |
| `part1.log` | C | 39.347 | 40.000 | 3.786 | 4500 | within |
| `part1.pulse` | C | 14.298 | 15.000 | 5.737 | 9999 | within |
| `part2.filter` | B | 16.699 | 25.000 | 6.990 | 7200 | within |
| `part2.sampler` | B | 39.888 | 100.000 | 4.066 | 1800 | within |

Verdict: **pass**
