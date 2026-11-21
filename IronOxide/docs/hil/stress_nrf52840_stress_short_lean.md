# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:fb5dace399febeb7027f7e74bc76f1234427301696929488ee94c53180d4626e

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.7 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 205 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 258379 compares left to the hardware, the closest 2 ticks ahead of the counter captured after the write (137 below 4 ticks, 140 below 8, 155 below 16); 6987 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8062 cycles (126.0 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.587 ms (37552 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.278 ms (17772 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.781 ms (369968 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11956 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2076 cycles (32.4 µs), mean 1836 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1905 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 24449 cycles (382.0 µs) over 5648608 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11955 response edges inside the run (target counted 11955) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 0.0 % of the run, the Kernel 100.0 % over 1 entries; runs per Task [0, 0, 0, 0, 0, 0, 0] |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 427 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 260 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8062 / 126.0 | 3181 | 2^12:17862 2^13:137 |
| 2 | 8999 | 7123 / 111.3 | 2767 | 2^12:8932 2^13:67 |
| 3 | 4499 | 7771 / 121.4 | 3058 | 2^12:4450 2^13:49 |
| 5 | 7199 | 8062 / 126.0 | 3157 | 2^12:7141 2^13:58 |
| 6 | 1799 | 7175 / 112.1 | 3339 | 2^12:1778 2^13:21 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.587 | 0 | 0 | 0 |
| 1 | 5999 | 0.278 | 0 | 0 | 0 |
| 2 | 8999 | 5.781 | 0 | 0 | 0 |
| 3 | 3375 | 4.346 | 0 | 0 | 0 |
| 4 | 11955 | 5.853 | 11956 | 2076 | 1836 |
| 5 | 6300 | 6.373 | 0 | 0 | 0 |
| 6 | 900 | 11.623 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18541 / 0.290 (min 8941, mean 10344) | 0 above the Budget; 2^14:17863 2^15:137 |
| Task 1 entry | 5999 | 9336 / 0.146 (min 3093, mean 3226) | 0 above the Budget; 2^12:5966 2^13:2 2^14:31 |
| Task 2 entry | 8999 | 343993 / 5.375 (min 334863, mean 336538) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204977 / 3.203 (min 191164, mean 195999) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11955 | 12754 / 0.199 (min 3170, mean 3521) | 0 above the Budget; 2^12:10850 2^13:1012 2^14:93 |
| Task 5 entry | 6300 | 37341 / 0.583 (min 20433, mean 26708) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 36194 / 0.566 (min 18470, mean 27645) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 9576 / 0.150 | |
| Task 0 on Resource 1 | 18000 | 9614 / 0.150 | |
| Task 2 on Resource 0 | 8999 | 327611 / 5.119 | |
| Task 2 on Resource 1 | 17998 | 6473 / 0.101 | |
| Task 5 on Resource 2 | 12600 | 28510 / 0.445 | |
| Task 6 on Resource 2 | 900 | 8379 / 0.131 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58453 | 2761 / 43.14 | 2547 |
| timer | 56931 | 4208 / 65.75 | 2482 |
| intake | 11956 | 2212 / 34.56 | 1953 |
| lock | 77397 | 296 / 4.62 | 295 |
| unlock | 76497 | 1576 / 24.62 | 1368 |
| job_end | 55528 | 1841 / 28.77 | 1511 |
| now | 5391779 | 153 / 2.39 | 152 |
| raise | 6000 | 1761 / 27.52 | 1699 |
| dma | 8999 | 558 / 8.72 | 387 |
| epilogue | 58453 | 1089 / 17.02 | 458 |
| restart | 900 | 4208 / 65.75 | 4194 |
| enforce | 2925 | 4208 / 65.75 | 3190 |
| view | 72189 | 327 / 5.11 | 292 |

Entries that interrupted an executing Job: Task 0: 2630, Task 1: 64, Task 2: 29816, Task 3: 6198, Task 4: 1231, Task 5: 3623, Task 6: 2727

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 7003 cycles; Task 1: 2 (timer, intake), 4544 cycles; Task 2: 5 (dispatch, timer, intake), 12148 cycles; Task 3: 3 (dispatch, timer, intake), 7338 cycles; Task 4: 3 (dispatch, timer), 8068 cycles; Task 5: 4 (dispatch, timer, intake), 9557 cycles; Task 6: 4 (dispatch, timer, intake), 9501 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3346 cycles
- Task 0 at 179940.000 ms, 2675 cycles
- Task 2 at 179945.000 ms, 2671 cycles
- Task 0 at 179950.000 ms, 3670 cycles
- Task 5 at 179950.000 ms, 3670 cycles
- Task 0 at 179960.000 ms, 2911 cycles
- Task 3 at 179960.000 ms, 2911 cycles
- Task 2 at 179965.000 ms, 2679 cycles
- Task 0 at 179970.000 ms, 3358 cycles
- Task 5 at 179975.000 ms, 2679 cycles
- Task 0 at 179980.000 ms, 2679 cycles
- Task 0 at 179990.000 ms, 2631 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5648608 of them, the longest 24449 cycles (382.0 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 5376585 |
| 512 to 1023 (8.0 to 16.0 µs) | 86238 |
| 1024 to 2047 (16.0 to 32.0 µs) | 70806 |
| 2048 to 4095 (32.0 to 64.0 µs) | 42924 |
| 4096 to 8191 (64.0 to 128.0 µs) | 60211 |
| 8192 to 16383 (128.0 to 256.0 µs) | 11819 |
| 16384 to 32767 (256.0 to 512.0 µs) | 25 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56541 | 24449 / 382.0 |
| timer | 25083 | 17468 / 272.9 |
| job_end | 5533664 | 4757 / 74.3 |
| epilogue | 33320 | 12973 / 202.7 |

The longest span's 9 entries, in order (cycles from the span's start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | now, exception 24 pending | 0 | 442 | 442 / 6.9 | 0 / 0.0 |
| 2 | timer, exception 24 pending | 606 | 3510 | 2904 / 45.4 | 164 / 2.6 |
| 3 | timer, enforced an Overrun, an unwind follows, exception 24 pending | 3714 | 7372 | 3658 / 57.2 | 204 / 3.2 |
| 4 | epilogue, exception 24 pending | 7538 | 8056 | 518 / 8.1 | 166 / 2.6 |
| 5 | timer, exception 22 pending | 8258 | 10944 | 2686 / 42.0 | 202 / 3.2 |
| 6 | intake, exception 40 pending | 11148 | 13558 | 2410 / 37.7 | 204 / 3.2 |
| 7 | dispatch, started a Job, exception 24 pending | 13762 | 16765 | 3003 / 46.9 | 204 / 3.2 |
| 8 | timer, exception 38 pending | 16952 | 21014 | 4062 / 63.5 | 187 / 2.9 |
| 9 | dispatch, started a Job | 21218 | 24449 | 3231 / 50.5 | 204 / 3.2 |

Rust halves 22914 cycles (358.0 µs), between them 1535 cycles (24.0 µs).

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| none | 20518912 | 1 | 205189.120 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short_lean/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short_lean/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=3375
pulse=11955
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=427
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=260
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
  detail=58387
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
  detail=289249
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
  detail=51746
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
  detail=288605
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
  detail=58291
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
  detail=52186
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
  detail=288685
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
  detail=58331
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
  detail=51698
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
  detail=289056
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
  detail=58379
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
  detail=288605
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
  detail=51730
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
     Finished in 8.39s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

Not judged: the records are not this build's
