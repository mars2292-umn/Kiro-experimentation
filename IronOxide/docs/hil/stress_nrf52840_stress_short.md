# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:e4e863f674d5f6b052555993608d0760939514fc11adb86c535f38b1491f827f

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 205 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 259915 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (40 below 4 ticks, 157 below 8, 199 below 16); 6498 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9208 cycles (143.9 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.668 ms (42756 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.362 ms (23200 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.931 ms (379592 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11949 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2202 cycles (34.4 µs), mean 1983 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2057 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 20980 cycles (327.8 µs) over 4585536 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 6945 cycles (108.5 µs) over 4678736 holds, closed by a epilogue entry; against the record's L_kernel 11000 cycles |
| responses under the storm | pass | 11948 response edges inside the run (target counted 11948) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 35.3 % of the run, the Kernel 43.7 % over 1346323 entries; runs per Task [18040, 5999, 20998, 6016, 12062, 7410, 1874] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23833 cycles (0.372 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 18040 runs for 18000 Jobs): the agent exceeds the trace beyond quantization, within a tenth |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 343 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 208 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 9208 / 143.9 | 3435 | 2^12:17854 2^13:133 2^14:12 |
| 2 | 8999 | 8132 / 127.1 | 3191 | 2^12:8577 2^13:422 |
| 3 | 4499 | 9208 / 143.9 | 3431 | 2^12:4407 2^13:80 2^14:12 |
| 5 | 7199 | 9208 / 143.9 | 3487 | 2^12:7043 2^13:150 2^14:6 |
| 6 | 1799 | 9208 / 143.9 | 3683 | 2^12:1772 2^13:23 2^14:4 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.668 | 0 | 0 | 0 |
| 1 | 5999 | 0.362 | 0 | 0 | 0 |
| 2 | 8999 | 5.931 | 0 | 0 | 0 |
| 3 | 3375 | 4.346 | 0 | 0 | 0 |
| 4 | 11948 | 5.942 | 11949 | 2202 | 1983 |
| 5 | 6300 | 6.601 | 0 | 0 | 0 |
| 6 | 900 | 11.910 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 23833 / 0.372 (min 10521, mean 12136) | 0 above the Budget; 2^14:17740 2^15:260 |
| Task 1 entry | 5999 | 10406 / 0.163 (min 3561, mean 3683) | 0 above the Budget; 2^12:5982 2^13:5 2^14:12 |
| Task 2 entry | 8999 | 350456 / 5.476 (min 337438, mean 339435) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 206752 / 3.231 (min 191224, mean 196828) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11948 | 17500 / 0.273 (min 3621, mean 4054) | 0 above the Budget; 2^12:10790 2^13:1055 2^14:102 2^15:1 |
| Task 5 entry | 6300 | 40431 / 0.632 (min 21375, mean 28738) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 41138 / 0.643 (min 22308, mean 31591) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 10774 / 0.168 | |
| Task 0 on Resource 1 | 18000 | 10803 / 0.169 | |
| Task 2 on Resource 0 | 8999 | 328813 / 5.138 | |
| Task 2 on Resource 1 | 17998 | 7265 / 0.114 | |
| Task 5 on Resource 2 | 12600 | 27023 / 0.422 | |
| Task 6 on Resource 2 | 900 | 4178 / 0.065 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58446 | 3094 / 48.34 | 2877 |
| timer | 57999 | 4855 / 75.86 | 2786 |
| intake | 11949 | 2334 / 36.47 | 2100 |
| lock | 77397 | 451 / 7.05 | 448 |
| unlock | 76497 | 1735 / 27.11 | 1539 |
| job_end | 55521 | 2004 / 31.31 | 1693 |
| now | 4325929 | 228 / 3.56 | 226 |
| raise | 6000 | 1936 / 30.25 | 1873 |
| dma | 8999 | 705 / 11.02 | 536 |
| epilogue | 58446 | 1265 / 19.77 | 625 |
| restart | 900 | 4855 / 75.86 | 4776 |
| enforce | 2925 | 4855 / 75.86 | 3568 |
| view | 72355 | 327 / 5.11 | 292 |

Entries that interrupted an executing Job: Task 0: 2906, Task 1: 29, Task 2: 29765, Task 3: 6497, Task 4: 1302, Task 5: 3677, Task 6: 2698

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 4 (timer, intake), 9820 cycles; Task 1: 2 (timer, intake), 4916 cycles; Task 2: 6 (dispatch, timer, intake), 16588 cycles; Task 3: 4 (dispatch, timer, intake), 10724 cycles; Task 4: 4 (dispatch, timer), 11508 cycles; Task 5: 4 (dispatch, timer, intake), 10496 cycles; Task 6: 4 (dispatch, timer, intake), 10564 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3362 cycles
- Task 0 at 179940.000 ms, 3000 cycles
- Task 2 at 179945.000 ms, 3004 cycles
- Task 0 at 179950.000 ms, 3814 cycles
- Task 5 at 179950.000 ms, 3814 cycles
- Task 0 at 179960.000 ms, 3244 cycles
- Task 3 at 179960.000 ms, 3244 cycles
- Task 2 at 179965.000 ms, 5820 cycles
- Task 0 at 179970.000 ms, 3750 cycles
- Task 5 at 179975.000 ms, 3016 cycles
- Task 0 at 179980.000 ms, 3008 cycles
- Task 0 at 179990.000 ms, 2952 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 4585536 of them, the longest 20980 cycles (327.8 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 4399111 |
| 1024 to 2047 (16.0 to 32.0 µs) | 65422 |
| 2048 to 4095 (32.0 to 64.0 µs) | 41589 |
| 4096 to 8191 (64.0 to 128.0 µs) | 65122 |
| 8192 to 16383 (128.0 to 256.0 µs) | 14124 |
| 16384 to 32767 (256.0 to 512.0 µs) | 168 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56537 | 20980 / 327.8 |
| timer | 26171 | 19553 / 305.5 |
| job_end | 4470535 | 2304 / 36.0 |
| epilogue | 32293 | 14727 / 230.1 |

The longest one's 7 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | job_end, an unwind follows, exception 56 pending | 0 | 2330 | 2330 / 36.4 | 0 / 0.0 |
| 2 | epilogue, exception 56 pending | 2514 | 3237 | 723 / 11.3 | 184 / 2.9 |
| 3 | dispatch, started a Job, exception 24 pending | 3521 | 6867 | 3346 / 52.3 | 284 / 4.4 |
| 4 | timer, exception 24 pending | 7151 | 10402 | 3251 / 50.8 | 284 / 4.4 |
| 5 | timer, exception 22 pending | 10703 | 13938 | 3235 / 50.5 | 301 / 4.7 |
| 6 | intake, exception 40 pending | 14239 | 17095 | 2856 / 44.6 | 301 / 4.7 |
| 7 | dispatch, started a Job | 17396 | 20980 | 3584 / 56.0 | 301 / 4.7 |

Rust halves 19325 cycles (302.0 µs), between them 1655 cycles (25.9 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 4678736 of them, the longest 6945 cycles (108.5 µs), closed by a epilogue entry:

The longest had 2 entries: it began with: timer, restarted a Partition, enforced an Overrun, an unwind follows; it ended with: epilogue.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3668 / 57.3 |
| timer | 5518 / 86.2 |
| job_end | 2304 / 36.0 |
| epilogue | 6945 / 108.5 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 313924 | 18040 | 0.350 |
| 1 | 25373 | 5999 | 0.150 |
| 2 | 4826444 | 20998 | 5.350 |
| 3 | 1549934 | 6016 | 4.560 |
| 4 | 56613 | 12062 | 0.180 |
| 5 | 349650 | 7410 | 0.860 |
| 6 | 125870 | 1874 | 1.000 |
| none | 13271104 | 44538 | 16617.480 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=3375
pulse=11948
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=343
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=208
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=0
dma_after_stop=0
spim0_enable=7
spim0_shorts=0
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=179301
  detail=58403
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179324
  detail=288711
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179324
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=52088
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179375
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179375
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179484
  detail=289217
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179484
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=58383
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179575
  detail=52100
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179575
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179575
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179644
  detail=288969
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179644
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=58403
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179775
  detail=52052
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179775
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179775
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179811
  detail=288886
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179811
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=58403
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179964
  detail=289217
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179964
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=52088
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179975
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179975
  detail=1
  prs_lo=128
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
expected_control=18000
expected_sensor=9000
part0_events=0
RSK-OK
Firmware exited successfully
     Finished in 8.44s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress_short.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` (sha256:e4e863f674d5f6b052555993608d0760939514fc11adb86c535f38b1491f827f) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:e4e863f674d5f6b052555993608d0760939514fc11adb86c535f38b1491f827f is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 564 cycles (measured, rsk-hil release-timing record, 77397 runs, max 451 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2169 cycles (measured, rsk-hil release-timing record, 76497 runs, max 1735 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3868 cycles (measured, rsk-hil release-timing record, 58446 runs, max 3094 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4087 cycles (measured, rsk-hil release-timing record, 55521 runs, max 3269 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6069 cycles (measured, rsk-hil release-timing record, 57999 runs, max 4855 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2753 cycles (measured, rsk-hil release-timing record, 11949 runs, max 2202 +25%, scenarios: nrf52840_stress_short (180 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 8682 cycles (measured, rsk-hil release-timing record, 4678736 runs, max 6945 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest hold of the run: the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler from its Rust half's beginning to its end with the unwind it runs before it returns (6945 cycles over 2 entries, closed by a epilogue entry; the longest single Rust half 4855; the longest run of entries back to back, each taken at an exception return and charged by the Analyzer as its own event's interference, 20980 cycles) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6069 cycles (measured, rsk-hil release-timing record, 57999 runs, max 4855 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6069 cycles (measured, rsk-hil release-timing record, 2925 runs, max 4855 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11510 cycles (measured, rsk-hil release-timing record, 40495 runs, max 9208 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 285 cycles (measured, rsk-hil release-timing record, 4325929 runs, max 228 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 882 cycles (measured, rsk-hil release-timing record, 8999 runs, max 705 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 409 cycles (measured, rsk-hil release-timing record, 72355 runs, max 327 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6069 cycles (measured, rsk-hil release-timing record, 900 runs, max 4855 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.253 | 10.000 | 0.668 | 18000 | within |
| `part0.echo` | A | 11.662 | 20.000 | 0.362 | 5999 | within |
| `part0.sensor` | A | 13.698 | 20.000 | 5.931 | 8999 | within |
| `part1.log` | C | 40.276 | 40.000 | 4.346 | 3375 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 14.666 | 15.000 | 5.942 | 11948 | within |
| `part2.filter` | B | 17.552 | 25.000 | 6.601 | 6300 | within |
| `part2.sampler` | B | 79.923 | 100.000 | 11.910 | 900 | within |

Verdict: **pass**
