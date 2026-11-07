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
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6915 cycles (108.0 µs) against J_rel 10000 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10658 rises and 10000 response edges in the capture); latency 90 to 5870 µs, median 90 µs, 0 later than 15 ms; Δ_stamp observed 5304 cycles (82.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 41.6 % of the run, the Kernel 35.2 % over 1159938 entries; runs per Task [18032, 5999, 20998, 5000, 10198, 7199, 1799] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20039 cycles (0.313 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18032 runs for 18000 Jobs) |

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
| 0 | 17999 | 6915 / 108.0 | 3553 | 2^12:17714 2^13:285 |
| 2 | 8999 | 5848 / 91.4 | 3216 | 2^12:8937 2^13:62 |
| 3 | 4499 | 6915 / 108.0 | 3574 | 2^12:4390 2^13:109 |
| 5 | 7199 | 6915 / 108.0 | 3575 | 2^12:7108 2^13:91 |
| 6 | 1799 | 6915 / 108.0 | 3863 | 2^12:1750 2^13:49 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.571 | 0 | 0 | 0 |
| 1 | 5999 | 0.231 | 0 | 0 | 0 |
| 2 | 8999 | 6.765 | 0 | 0 | 0 |
| 3 | 4500 | 3.795 | 0 | 0 | 0 |
| 4 | 9999 | 5.892 | 10000 | 2142 | 1974 |
| 5 | 7200 | 7.002 | 0 | 0 | 0 |
| 6 | 1800 | 4.074 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20039 / 0.313 (min 8682, mean 11742) | 0 above the Budget; 2^14:17951 2^15:49 |
| Task 1 entry | 5999 | 6659 / 0.104 (min 2998, mean 3086) | 0 above the Budget; 2^12:5995 2^13:4 |
| Task 2 entry | 8999 | 405476 / 6.336 (min 397952, mean 399791) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 199111 / 3.111 (min 190225, mean 195200) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12041 / 0.188 (min 3055, mean 3167) | 0 above the Budget; 2^12:9890 2^13:9 2^14:100 |
| Task 5 entry | 7200 | 9501 / 0.148 (min 5909, mean 6020) | 0 above the Budget; 2^13:7138 2^14:62 |
| Task 6 entry | 1800 | 17020 / 0.266 (min 6748, mean 8463) | 0 above the Budget; 2^13:932 2^14:867 2^15:1 |
| Task 0 on Resource 0 | 18000 | 3836 / 0.060 | |
| Task 0 on Resource 1 | 18000 | 7525 / 0.118 | |
| Task 2 on Resource 0 | 8999 | 388749 / 6.074 | |
| Task 2 on Resource 1 | 17998 | 3773 / 0.059 | |
| Task 5 on Resource 2 | 7200 | 3752 / 0.059 | |
| Task 6 on Resource 2 | 1800 | 3772 / 0.059 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2848 / 44.50 | 2704 |
| timer | 55017 | 3852 / 60.19 | 2994 |
| intake | 10000 | 2267 / 35.42 | 2099 |
| lock | 71997 | 463 / 7.23 | 461 |
| unlock | 71997 | 1805 / 28.20 | 1621 |
| job_end | 56497 | 2016 / 31.50 | 1755 |
| now | 7727004 | 242 / 3.78 | 240 |
| raise | 6000 | 2076 / 32.44 | 1967 |
| dma | 1808 | 703 / 10.98 | 491 |
| epilogue | 56497 | 1211 / 18.92 | 568 |
| view | 69195 | 330 / 5.16 | 295 |

Entries that interrupted an executing Job: Task 0: 10552, Task 1: 4, Task 2: 29965, Task 3: 2170, Task 4: 309, Task 5: 62, Task 6: 868

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3635 cycles
- Task 0 at 179940.000 ms, 3156 cycles
- Task 2 at 179945.000 ms, 3156 cycles
- Task 0 at 179950.000 ms, 3859 cycles
- Task 5 at 179950.000 ms, 3859 cycles
- Task 0 at 179960.000 ms, 3388 cycles
- Task 3 at 179960.000 ms, 3388 cycles
- Task 2 at 179965.000 ms, 3156 cycles
- Task 0 at 179970.000 ms, 3767 cycles
- Task 5 at 179975.000 ms, 3152 cycles
- Task 0 at 179980.000 ms, 3156 cycles
- Task 0 at 179990.000 ms, 3044 cycles

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 310180 | 18032 | 0.300 |
| 1 | 22390 | 5999 | 0.100 |
| 2 | 5668107 | 20998 | 6.220 |
| 3 | 1368700 | 5000 | 3.100 |
| 4 | 38057 | 10198 | 0.140 |
| 5 | 58598 | 7199 | 0.140 |
| 6 | 21404 | 1799 | 0.150 |
| none | 10508869 | 43791 | 8.580 |

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
     Finished in 7.80s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` (sha256:91e83a0efd889a5000e931499fa98874726cb3b306a6038fd4f6d0fef5ba83bf) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:91e83a0efd889a5000e931499fa98874726cb3b306a6038fd4f6d0fef5ba83bf is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,plain-measure)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 579 cycles (measured, rsk-hil release-timing record, 71997 runs, max 463 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2257 cycles (measured, rsk-hil release-timing record, 71997 runs, max 1805 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3560 cycles (measured, rsk-hil release-timing record, 56497 runs, max 2848 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4034 cycles (measured, rsk-hil release-timing record, 56497 runs, max 3227 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 4815 cycles (measured, rsk-hil release-timing record, 55017 runs, max 3852 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stamp: 6630 cycles (measured, rsk-hil release-timing record, 10000 runs, max 5304 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: from the Release_Source edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3]), to the intake handler's timestamp of the event: the masking of any Kernel entry in progress, the exception entry, and the handler's own read (1326 ticks at 16000000 per cycle group of 4; the Kernel's record, task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 4815 cycles (measured, rsk-hil release-timing record, 8182509 runs, max 3852 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the longest Kernel entry of the run (every entry masks preemption, DD-01): the largest Rust half of any kind (the stub's cycles outside the charge points, Δ_entry, come from the cycle configuration's record) (task 19.3; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 4815 cycles (measured, rsk-hil release-timing record, 55017 runs, max 3852 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 8644 cycles (measured, rsk-hil release-timing record, 40495 runs, max 6915 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 303 cycles (measured, rsk-hil release-timing record, 7727004 runs, max 242 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 879 cycles (measured, rsk-hil release-timing record, 1808 runs, max 703 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 413 cycles (measured, rsk-hil release-timing record, 69195 runs, max 330 +25%, scenarios: nrf52840_measure (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (1799 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 95 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.198 | 10.000 | 1.571 | 18000 | within |
| `part0.echo` | A | 9.974 | 20.000 | 0.231 | 5999 | within |
| `part0.sensor` | A | 13.182 | 20.000 | 6.765 | 8999 | within |
| `part1.log` | C | 39.038 | 40.000 | 3.795 | 4500 | within |
| `part1.pulse` | C | 14.158 | 15.000 | 5.892 | 9999 | within |
| `part2.filter` | B | 16.515 | 25.000 | 7.002 | 7200 | within |
| `part2.sampler` | B | 39.533 | 100.000 | 4.074 | 1800 | within |

Verdict: **pass**
