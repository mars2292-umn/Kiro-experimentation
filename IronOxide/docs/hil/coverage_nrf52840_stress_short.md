# HIL coverage run (task 19.2, R34.4) over the stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:389e481131179e0431b22ecb03d7136ba6baf693384922d923fe5276860709ef

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 206 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 259897 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (7 below 4 ticks, 28 below 8, 41 below 16); 6516 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8848 cycles (138.2 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.673 ms (43056 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.311 ms (19892 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.935 ms (379856 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11949 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2197 cycles (34.3 µs), mean 1978 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2051 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 23513 cycles (367.4 µs) over 4106378 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 7046 cycles (110.1 µs) over 4194400 holds, closed by a epilogue entry; against the record's L_kernel 11000 cycles |
| responses under the storm | pass | 11948 response edges inside the run (target counted 11948) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 35.4 % of the run, the Kernel 41.5 % over 1867763 entries; runs per Task [18038, 6002, 20999, 6035, 12030, 7432, 1880] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24233 cycles (0.379 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23040 cycles (0.360 ms at 100000 samples/s; 18038 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 303 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 185 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8848 / 138.2 | 3401 | 2^12:17841 2^13:141 2^14:17 |
| 2 | 8999 | 8237 / 128.7 | 3214 | 2^12:8579 2^13:419 2^14:1 |
| 3 | 4499 | 8815 / 137.7 | 3423 | 2^12:4422 2^13:68 2^14:9 |
| 5 | 7199 | 8815 / 137.7 | 3454 | 2^12:7047 2^13:148 2^14:4 |
| 6 | 1799 | 8815 / 137.7 | 3690 | 2^12:1773 2^13:22 2^14:4 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.673 | 0 | 0 | 0 |
| 1 | 5999 | 0.311 | 0 | 0 | 0 |
| 2 | 8999 | 5.935 | 0 | 0 | 0 |
| 3 | 3375 | 4.523 | 0 | 0 | 0 |
| 4 | 11948 | 6.020 | 11949 | 2197 | 1978 |
| 5 | 6300 | 6.605 | 0 | 0 | 0 |
| 6 | 900 | 11.984 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 24233 / 0.379 (min 10797, mean 12467) | 0 above the Budget; 2^14:17766 2^15:234 |
| Task 1 entry | 5999 | 10598 / 0.166 (min 3713, mean 3835) | 0 above the Budget; 2^12:5980 2^13:3 2^14:16 |
| Task 2 entry | 8999 | 348701 / 5.448 (min 337781, mean 339786) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 207479 / 3.242 (min 191167, mean 197283) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11948 | 14401 / 0.225 (min 3847, mean 4260) | 0 above the Budget; 2^12:10755 2^13:1123 2^14:70 |
| Task 5 entry | 6300 | 41492 / 0.648 (min 21716, mean 29921) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 41258 / 0.645 (min 23333, mean 31723) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 10915 / 0.171 | |
| Task 0 on Resource 1 | 18000 | 10857 / 0.170 | |
| Task 2 on Resource 0 | 8999 | 329031 / 5.141 | |
| Task 2 on Resource 1 | 17998 | 7354 / 0.115 | |
| Task 5 on Resource 2 | 12600 | 27670 / 0.432 | |
| Task 6 on Resource 2 | 900 | 779 / 0.012 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58447 | 3113 / 48.64 | 2892 |
| timer | 57999 | 4946 / 77.28 | 2796 |
| intake | 11949 | 2323 / 36.30 | 2090 |
| lock | 77397 | 465 / 7.27 | 462 |
| unlock | 76497 | 1732 / 27.06 | 1536 |
| job_end | 55521 | 1965 / 30.70 | 1681 |
| now | 3841592 | 227 / 3.55 | 226 |
| raise | 6000 | 1918 / 29.97 | 1853 |
| dma | 8999 | 834 / 13.03 | 566 |
| epilogue | 58446 | 1265 / 19.77 | 627 |
| restart | 900 | 4946 / 77.28 | 4752 |
| enforce | 2925 | 4946 / 77.28 | 3575 |
| view | 72365 | 311 / 4.86 | 280 |

Entries that interrupted an executing Job: Task 0: 2953, Task 1: 34, Task 2: 29649, Task 3: 6536, Task 4: 1232, Task 5: 4628, Task 6: 2722

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 4 (timer, intake), 9898 cycles; Task 1: 2 (timer, intake), 4982 cycles; Task 2: 5 (dispatch, timer, intake), 13565 cycles; Task 3: 3 (dispatch, timer, intake), 8122 cycles; Task 4: 3 (dispatch, timer), 8931 cycles; Task 5: 4 (dispatch, timer, intake), 10595 cycles; Task 6: 4 (dispatch, timer, intake), 10595 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3920 cycles
- Task 0 at 179940.000 ms, 3003 cycles
- Task 2 at 179945.000 ms, 3007 cycles
- Task 0 at 179950.000 ms, 3568 cycles
- Task 5 at 179950.000 ms, 3568 cycles
- Task 0 at 179960.000 ms, 3243 cycles
- Task 3 at 179960.000 ms, 3243 cycles
- Task 2 at 179965.000 ms, 6351 cycles
- Task 0 at 179970.000 ms, 3824 cycles
- Task 5 at 179975.000 ms, 3023 cycles
- Task 0 at 179980.000 ms, 3011 cycles
- Task 0 at 179990.000 ms, 2955 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 4106378 of them, the longest 23513 cycles (367.4 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 3916866 |
| 1024 to 2047 (16.0 to 32.0 µs) | 67438 |
| 2048 to 4095 (32.0 to 64.0 µs) | 46749 |
| 4096 to 8191 (64.0 to 128.0 µs) | 59641 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15558 |
| 16384 to 32767 (256.0 to 512.0 µs) | 126 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56574 | 23513 / 367.4 |
| timer | 26181 | 19682 / 307.5 |
| job_end | 3990313 | 2297 / 35.9 |
| epilogue | 33310 | 14715 / 229.9 |

The longest one's 8 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | now, exception 24 pending | 0 | 533 | 533 / 8.3 | 0 / 0.0 |
| 2 | timer, enforced an Overrun, an unwind follows, exception 24 pending | 788 | 4763 | 3975 / 62.1 | 255 / 4.0 |
| 3 | epilogue, exception 24 pending | 4989 | 5687 | 698 / 10.9 | 226 / 3.5 |
| 4 | timer, exception 22 pending | 5972 | 9004 | 3032 / 47.4 | 285 / 4.5 |
| 5 | intake, exception 40 pending | 9292 | 11867 | 2575 / 40.2 | 288 / 4.5 |
| 6 | dispatch, started a Job, exception 24 pending | 12155 | 15526 | 3371 / 52.7 | 288 / 4.5 |
| 7 | timer, exception 38 pending | 15810 | 19645 | 3835 / 59.9 | 284 / 4.4 |
| 8 | dispatch, started a Job | 19946 | 23513 | 3567 / 55.7 | 301 / 4.7 |

Rust halves 21586 cycles (337.3 µs), between them 1927 cycles (30.1 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 4194400 of them, the longest 7046 cycles (110.1 µs), closed by a epilogue entry:

The longest had 2 entries: it began with: timer, restarted a Partition, enforced an Overrun, an unwind follows, exception 22 pending; it ended with: epilogue, exception 22 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3711 / 58.0 |
| timer | 5550 / 86.7 |
| job_end | 2297 / 35.9 |
| epilogue | 7046 / 110.1 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 321833 | 18038 | 0.360 |
| 1 | 26301 | 6002 | 0.160 |
| 2 | 4831306 | 20999 | 5.310 |
| 3 | 1552073 | 6035 | 4.560 |
| 4 | 59530 | 12030 | 0.160 |
| 5 | 360787 | 7432 | 0.870 |
| 6 | 126198 | 1880 | 1.000 |
| none | 13305396 | 44528 | 16564.000 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_stress_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo

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
loop:part0:sensor:hold=303
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=185
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
  detail=58534
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
  detail=288764
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
  detail=51980
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
  detail=289263
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
  detail=58510
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
  detail=51996
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
  detail=288651
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
  detail=58538
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
  detail=51944
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
  detail=289114
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
  detail=58534
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
  detail=289263
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
  detail=51980
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
     Finished in 9.12s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 90 % (20 of 22 branch outcomes), part1.pulse 100 % (10 of 10 branch outcomes), part2.filter 69 % (18 of 26 branch outcomes), part2.sampler 58 % (14 of 24 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 6 | 4 | 4 | 100 % | pass |
| `part0.echo` | 4 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 8 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 8 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 8 | 6 | 5 | 83 % | pass |
| `part1.log` | 9 | 22 | 20 | 90 % | pass |
| `part1.pulse` | 7 | 10 | 10 | 100 % | pass |
| `part2.filter` | 19 | 26 | 18 | 69 % | pass |
| `part2.filter#cs:stats` | 19 | 26 | 18 | 69 % | pass |
| `part2.sampler` | 18 | 24 | 14 | 58 % | pass |
| `part2.sampler#cs:stats` | 18 | 24 | 14 | 58 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::traffic_step` | 10 | 5 | 184:8 true, 186:12 true, 186:12 false, 196:12 true, 219:20 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 2 | 1 | 210:16 false |
| `p1_demo_part2::Stats::record` | 2 | 1 | 423:12 false |
| `p1_demo_part2::filter` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 6 | 6 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 4 | 4 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 2 | 0 | 699:34 true, 699:34 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::validate_req` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 8 | 8 |  |
| `p1_demo_part1::log::{closure}` | 2 | 0 | 415:38 true, 415:38 false |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::inject::spin_ms` | 4 | 4 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 405:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
