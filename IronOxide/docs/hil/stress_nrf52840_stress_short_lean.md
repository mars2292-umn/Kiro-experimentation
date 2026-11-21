# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:9c986ca35fdb258b5f581b4a30f2ef5c4f36b748e9979c64c40e36dd94e8ea29

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 205 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 259277 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (11 below 4 ticks, 17 below 8, 40 below 16); 6064 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8122 cycles (126.9 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.597 ms (38204 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.282 ms (18044 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.802 ms (371332 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11955 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2076 cycles (32.4 µs), mean 1830 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1899 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 21258 cycles (332.2 µs) over 5187383 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 6312 cycles (98.6 µs) over 5281569 holds, closed by a epilogue entry; against the record's L_kernel 11000 cycles |
| responses under the storm | pass | 11954 response edges inside the run (target counted 11954) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 0.0 % of the run, the Kernel 100.0 % over 1 entries; runs per Task [0, 0, 0, 0, 0, 0, 0] |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 391 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 237 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8122 / 126.9 | 3119 | 2^12:17876 2^13:123 |
| 2 | 8999 | 7958 / 124.3 | 2813 | 2^12:8904 2^13:95 |
| 3 | 4499 | 7871 / 123.0 | 3098 | 2^12:4453 2^13:46 |
| 5 | 7199 | 7835 / 122.4 | 3147 | 2^12:7145 2^13:54 |
| 6 | 1799 | 7603 / 118.8 | 3387 | 2^12:1781 2^13:18 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.597 | 0 | 0 | 0 |
| 1 | 5999 | 0.282 | 0 | 0 | 0 |
| 2 | 8999 | 5.802 | 0 | 0 | 0 |
| 3 | 3375 | 4.376 | 0 | 0 | 0 |
| 4 | 11954 | 5.853 | 11955 | 2076 | 1830 |
| 5 | 6300 | 6.408 | 0 | 0 | 0 |
| 6 | 900 | 11.690 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19036 / 0.297 (min 9249, mean 10746) | 0 above the Budget; 2^14:17880 2^15:120 |
| Task 1 entry | 5999 | 9501 / 0.148 (min 3142, mean 3265) | 0 above the Budget; 2^12:5984 2^13:2 2^14:13 |
| Task 2 entry | 8999 | 344857 / 5.388 (min 335457, mean 337290) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 205676 / 3.214 (min 191536, mean 196334) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11954 | 12304 / 0.192 (min 3224, mean 3562) | 0 above the Budget; 2^12:10879 2^13:1027 2^14:48 |
| Task 5 entry | 6300 | 35395 / 0.553 (min 20707, mean 26911) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 37364 / 0.584 (min 19394, mean 28551) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 9794 / 0.153 | |
| Task 0 on Resource 1 | 18000 | 9941 / 0.155 | |
| Task 2 on Resource 0 | 8999 | 327966 / 5.124 | |
| Task 2 on Resource 1 | 17998 | 6730 / 0.105 | |
| Task 5 on Resource 2 | 12600 | 25910 / 0.405 | |
| Task 6 on Resource 2 | 900 | 3794 / 0.059 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58452 | 2761 / 43.14 | 2548 |
| timer | 56909 | 4392 / 68.62 | 2486 |
| intake | 11955 | 2212 / 34.56 | 1948 |
| lock | 77397 | 282 / 4.41 | 281 |
| unlock | 76497 | 1583 / 24.73 | 1362 |
| job_end | 55527 | 1818 / 28.41 | 1511 |
| now | 4929834 | 155 / 2.42 | 153 |
| raise | 6000 | 1755 / 27.42 | 1698 |
| dma | 8999 | 562 / 8.78 | 388 |
| epilogue | 58452 | 1093 / 17.08 | 456 |
| restart | 900 | 4392 / 68.62 | 4189 |
| enforce | 2925 | 4392 / 68.62 | 3189 |
| view | 72124 | 327 / 5.11 | 292 |

Entries that interrupted an executing Job: Task 0: 2836, Task 1: 28, Task 2: 29685, Task 3: 6157, Task 4: 1138, Task 5: 3624, Task 6: 2714

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 6951 cycles; Task 1: 2 (timer, intake), 4500 cycles; Task 2: 5 (dispatch, timer, intake), 11660 cycles; Task 3: 3 (dispatch, timer, intake), 7342 cycles; Task 4: 3 (dispatch, timer), 7636 cycles; Task 5: 4 (dispatch, timer, intake), 9465 cycles; Task 6: 4 (dispatch, timer, intake), 9501 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3538 cycles
- Task 0 at 179940.000 ms, 2703 cycles
- Task 2 at 179945.000 ms, 2703 cycles
- Task 0 at 179950.000 ms, 3478 cycles
- Task 5 at 179950.000 ms, 3478 cycles
- Task 0 at 179960.000 ms, 2943 cycles
- Task 3 at 179960.000 ms, 2943 cycles
- Task 2 at 179965.000 ms, 2703 cycles
- Task 0 at 179970.000 ms, 3206 cycles
- Task 5 at 179975.000 ms, 2691 cycles
- Task 0 at 179980.000 ms, 2695 cycles
- Task 0 at 179990.000 ms, 2623 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 5187383 of them, the longest 21258 cycles (332.2 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4890492 |
| 512 to 1023 (8.0 to 16.0 µs) | 111281 |
| 1024 to 2047 (16.0 to 32.0 µs) | 70995 |
| 2048 to 4095 (32.0 to 64.0 µs) | 42275 |
| 4096 to 8191 (64.0 to 128.0 µs) | 59497 |
| 8192 to 16383 (128.0 to 256.0 µs) | 12812 |
| 16384 to 32767 (256.0 to 512.0 µs) | 31 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56632 | 21258 / 332.2 |
| timer | 24363 | 14842 / 231.9 |
| job_end | 5072883 | 4977 / 77.8 |
| epilogue | 33505 | 11535 / 180.2 |

The longest one's 8 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | now, exception 24 pending | 0 | 468 | 468 / 7.3 | 0 / 0.0 |
| 2 | timer, enforced an Overrun, an unwind follows, exception 24 pending | 688 | 4367 | 3679 / 57.5 | 220 / 3.4 |
| 3 | epilogue, exception 24 pending | 4562 | 5095 | 533 / 8.3 | 195 / 3.0 |
| 4 | timer, exception 22 pending | 5351 | 8053 | 2702 / 42.2 | 256 / 4.0 |
| 5 | intake, exception 40 pending | 8313 | 10765 | 2452 / 38.3 | 260 / 4.1 |
| 6 | dispatch, started a Job, exception 24 pending | 11025 | 14046 | 3021 / 47.2 | 260 / 4.1 |
| 7 | timer, exception 38 pending | 14289 | 17812 | 3523 / 55.0 | 243 / 3.8 |
| 8 | dispatch, started a Job | 18072 | 21258 | 3186 / 49.8 | 260 / 4.1 |

Rust halves 19564 cycles (305.7 µs), between them 1694 cycles (26.5 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 5281569 of them, the longest 6312 cycles (98.6 µs), closed by a epilogue entry:

The longest had 2 entries: it began with: timer, restarted a Partition, enforced an Overrun, an unwind follows, exception 22 pending; it ended with: epilogue, exception 22 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3355 / 52.4 |
| timer | 5147 / 80.4 |
| job_end | 2121 / 33.1 |
| epilogue | 6312 / 98.6 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| none | 20516864 | 1 | 205168.640 |

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
pulse=11954
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=391
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=237
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
  detail=58643
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
  detail=288809
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
  detail=52082
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
  detail=288789
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
  detail=58611
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
  detail=52082
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
  detail=288783
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
  detail=58711
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
  detail=52066
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
  detail=288720
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
  detail=58643
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
  detail=288789
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
  detail=52082
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

Not judged: the records are not this build's
