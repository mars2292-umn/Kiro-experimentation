# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:f5ee2ab6b53bea16a538cc9f9f7ab3c670704196690cedfbf4e366c98c553dce

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3685 s (operating duration 3660 s) |
| compare arms (HW-12, ASM-19) | pass | 5279393 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (504 below 4 ticks, 818 below 8, 1727 below 16); 137340 pended by software, the instant reached |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9236 cycles (144.3 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 0.688 ms (44060 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 121999 Jobs, response at most 0.361 ms (23116 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 182999 Jobs, response at most 5.932 ms (379668 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 242941 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2202 cycles (34.4 µs), mean 1984 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2057 % of the run |
| Kernel events | pass | 137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 54900 actions recorded (32 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 26010 cycles (406.4 µs) over 93209697 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 7044 cycles (110.1 µs) over 95136127 holds, closed by a epilogue entry; against the record's L_kernel 11000 cycles |
| responses under the storm | pass | 242940 response edges inside the run (target counted 242940) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.0 % of the run, the Kernel 36.3 % over 27391158 entries; runs per Task [366827, 122040, 427024, 122058, 245036, 150885, 38372] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24590 cycles (0.384 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23680 cycles (0.370 ms at 100000 samples/s; 366827 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 343 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 208 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 9236 / 144.3 | 3469 | 2^12:362286 2^13:3355 2^14:358 |
| 2 | 182999 | 9152 / 143.0 | 3204 | 2^12:174593 2^13:8295 2^14:111 |
| 3 | 91499 | 9236 / 144.3 | 3410 | 2^12:90149 2^13:1200 2^14:150 |
| 5 | 146399 | 9236 / 144.3 | 3508 | 2^12:142689 2^13:3529 2^14:181 |
| 6 | 36599 | 9236 / 144.3 | 3677 | 2^12:36102 2^13:463 2^14:34 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 0.688 | 0 | 0 | 0 |
| 1 | 121999 | 0.361 | 0 | 0 | 0 |
| 2 | 182999 | 5.932 | 0 | 0 | 0 |
| 3 | 68625 | 4.469 | 0 | 0 | 0 |
| 4 | 242940 | 6.075 | 242941 | 2202 | 1984 |
| 5 | 128100 | 6.603 | 0 | 0 | 0 |
| 6 | 18300 | 12.000 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 24590 / 0.384 (min 10521, mean 12107) | 0 above the Budget; 2^14:361190 2^15:4810 |
| Task 1 entry | 121999 | 10482 / 0.164 (min 3561, mean 3687) | 0 above the Budget; 2^12:121612 2^13:50 2^14:337 |
| Task 2 entry | 182999 | 350156 / 5.471 (min 337366, mean 339542) | 0 above the Budget; 2^15:182999 |
| Task 3 entry | 68625 | 206992 / 3.234 (min 188320, mean 196941) | 0 above the Budget; 2^15:68625 |
| Task 4 entry | 242940 | 17819 / 0.278 (min 3641, mean 4046) | 0 above the Budget; 2^12:219545 2^13:21576 2^14:1817 2^15:2 |
| Task 5 entry | 128100 | 40779 / 0.637 (min 21287, mean 28751) | 0 above the Budget; 2^15:128100 |
| Task 6 entry | 18300 | 44650 / 0.698 (min 22308, mean 31680) | 0 above the Budget; 2^15:18300 |
| Task 0 on Resource 0 | 366000 | 10914 / 0.171 | |
| Task 0 on Resource 1 | 366000 | 14379 / 0.225 | |
| Task 2 on Resource 0 | 182999 | 329617 / 5.150 | |
| Task 2 on Resource 1 | 365998 | 7321 / 0.114 | |
| Task 5 on Resource 2 | 256200 | 27250 / 0.426 | |
| Task 6 on Resource 2 | 18300 | 9406 / 0.147 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1188440 | 3094 / 48.34 | 2877 |
| timer | 1178893 | 4959 / 77.48 | 2786 |
| intake | 242941 | 2334 / 36.47 | 2100 |
| lock | 1573797 | 451 / 7.05 | 448 |
| unlock | 1555497 | 1743 / 27.23 | 1539 |
| job_end | 1128963 | 2004 / 31.31 | 1693 |
| now | 87962598 | 228 / 3.56 | 226 |
| raise | 122000 | 1938 / 30.28 | 1873 |
| dma | 182999 | 705 / 11.02 | 536 |
| epilogue | 1188438 | 1265 / 19.77 | 625 |
| restart | 18300 | 4959 / 77.48 | 4772 |
| enforce | 59475 | 4959 / 77.48 | 3565 |
| view | 1471262 | 327 / 5.11 | 292 |

Entries that interrupted an executing Job: Task 0: 56050, Task 1: 724, Task 2: 607230, Task 3: 131390, Task 4: 25841, Task 5: 75581, Task 6: 55741

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 4 (timer, intake), 10204 cycles; Task 1: 2 (timer, intake), 4936 cycles; Task 2: 6 (dispatch, timer, intake), 16296 cycles; Task 3: 3 (dispatch, timer, intake), 8078 cycles; Task 4: 4 (dispatch, timer), 11540 cycles; Task 5: 4 (dispatch, timer, intake), 10620 cycles; Task 6: 5 (dispatch, timer, intake), 13210 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 3650 cycles
- Task 0 at 3659940.000 ms, 3008 cycles
- Task 2 at 3659945.000 ms, 3004 cycles
- Task 0 at 3659950.000 ms, 3894 cycles
- Task 5 at 3659950.000 ms, 3894 cycles
- Task 0 at 3659960.000 ms, 3244 cycles
- Task 3 at 3659960.000 ms, 3244 cycles
- Task 2 at 3659965.000 ms, 6004 cycles
- Task 0 at 3659970.000 ms, 3874 cycles
- Task 5 at 3659975.000 ms, 3016 cycles
- Task 0 at 3659980.000 ms, 3008 cycles
- Task 0 at 3659990.000 ms, 2924 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 93209697 of them, the longest 26010 cycles (406.4 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 89420774 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1330581 |
| 2048 to 4095 (32.0 to 64.0 µs) | 822064 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1343970 |
| 8192 to 16383 (128.0 to 256.0 µs) | 289024 |
| 16384 to 32767 (256.0 to 512.0 µs) | 3284 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 1149486 | 26010 / 406.4 |
| timer | 531785 | 22239 / 347.5 |
| job_end | 90873278 | 5593 / 87.4 |
| epilogue | 655148 | 14725 / 230.1 |

The longest one's 8 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | now, exception 24 pending | 0 | 533 | 533 / 8.3 | 0 / 0.0 |
| 2 | timer, exception 24 pending | 788 | 3996 | 3208 / 50.1 | 255 / 4.0 |
| 3 | timer, exception 24 pending | 4297 | 7396 | 3099 / 48.4 | 301 / 4.7 |
| 4 | timer, exception 22 pending | 7697 | 10936 | 3239 / 50.6 | 301 / 4.7 |
| 5 | intake, exception 40 pending | 11237 | 14101 | 2864 / 44.8 | 301 / 4.7 |
| 6 | dispatch, started a Job, exception 24 pending | 14402 | 17957 | 3555 / 55.5 | 301 / 4.7 |
| 7 | timer, exception 38 pending | 18241 | 22167 | 3926 / 61.3 | 284 / 4.4 |
| 8 | dispatch, started a Job | 22468 | 26010 | 3542 / 55.3 | 301 / 4.7 |

Rust halves 23966 cycles (374.5 µs), between them 2044 cycles (31.9 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 95136127 of them, the longest 7044 cycles (110.1 µs), closed by a epilogue entry:

The longest had 2 entries: it began with: timer, restarted a Partition, enforced an Overrun, an unwind follows, exception 22 pending; it ended with: epilogue, exception 22 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3678 / 57.5 |
| timer | 5915 / 92.4 |
| job_end | 2304 / 36.0 |
| epilogue | 7044 / 110.1 |

## The Kernel's events

137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6366346 | 366827 | 0.370 |
| 1 | 516867 | 122040 | 0.150 |
| 2 | 98178422 | 427024 | 5.340 |
| 3 | 31525591 | 122058 | 4.650 |
| 4 | 1147341 | 245036 | 0.190 |
| 5 | 7112630 | 150885 | 0.890 |
| 6 | 2563904 | 38372 | 1.010 |
| none | 221114211 | 905615 | 16689.660 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182999
log=68625
pulse=242940
echo=121999
blocked_releases=182999
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
  at_ms=3659301
  detail=58403
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659324
  detail=289261
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659324
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659375
  detail=52032
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659375
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659375
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659484
  detail=289241
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659484
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659501
  detail=58367
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659575
  detail=52032
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659575
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659575
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659644
  detail=288919
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659644
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659701
  detail=66177
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659775
  detail=52008
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659775
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659775
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659811
  detail=289430
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659811
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659901
  detail=58403
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659964
  detail=289241
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659964
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659975
  detail=52032
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659975
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659975
  detail=1
  prs_lo=128
  prs_hi=0
event=10
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=366000
expected_sensor=183000
part0_events=0
RSK-OK
Firmware exited successfully
     Finished in 8.44s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress.json): exit 2. 

Not judged: the Analyzer rejected the inputs: [R28.4] the WCET of Task `part2.filter` (51239 cycles) exceeds its Budget (51200 cycles)
