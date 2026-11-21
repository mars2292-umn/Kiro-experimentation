# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:747f11bb4f0b33a87c7352d3593d0e3462e4a7511bb20d73caedd7a996fe6206

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 259126 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (20 below 4 ticks, 27 below 8, 65 below 16); 6247 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8025 cycles (125.4 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.571 ms (36524 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.269 ms (17220 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.748 ms (367848 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11955 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2055 cycles (32.1 µs), mean 1816 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1885 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 2 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 20451 cycles (319.5 µs) over 6485242 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11954 response edges inside the run (target counted 11954) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 0.0 % of the run, the Kernel 100.0 % over 1 entries; runs per Task [0, 0, 0, 0, 0, 0, 0] |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 494 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 300 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8025 / 125.4 | 3125 | 2^12:17896 2^13:103 |
| 2 | 8999 | 7652 / 119.6 | 2963 | 2^12:8829 2^13:170 |
| 3 | 4499 | 7412 / 115.8 | 3250 | 2^12:4444 2^13:55 |
| 5 | 7199 | 7652 / 119.6 | 3236 | 2^12:7117 2^13:82 |
| 6 | 1799 | 7140 / 111.6 | 3557 | 2^12:1770 2^13:29 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.571 | 0 | 0 | 0 |
| 1 | 5999 | 0.269 | 0 | 0 | 0 |
| 2 | 8999 | 5.748 | 0 | 0 | 0 |
| 3 | 3375 | 4.305 | 0 | 0 | 0 |
| 4 | 11954 | 5.782 | 11955 | 2055 | 1816 |
| 5 | 6300 | 6.290 | 0 | 0 | 0 |
| 6 | 900 | 11.454 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18021 / 0.282 (min 8459, mean 9911) | 0 above the Budget; 2^14:17885 2^15:115 |
| Task 1 entry | 5999 | 8995 / 0.141 (min 2970, mean 3085) | 0 above the Budget; 2^12:5992 2^13:3 2^14:4 |
| Task 2 entry | 8999 | 342927 / 5.358 (min 334211, mean 335828) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204962 / 3.203 (min 188318, mean 196218) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11954 | 12424 / 0.194 (min 3040, mean 3410) | 0 above the Budget; 2^12:10848 2^13:1006 2^14:100 |
| Task 5 entry | 6300 | 36391 / 0.569 (min 20219, mean 25789) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 35013 / 0.547 (min 17301, mean 26559) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 9608 / 0.150 | |
| Task 0 on Resource 1 | 18000 | 9438 / 0.147 | |
| Task 2 on Resource 0 | 8999 | 327545 / 5.118 | |
| Task 2 on Resource 1 | 17998 | 6371 / 0.100 | |
| Task 5 on Resource 2 | 12600 | 25039 / 0.391 | |
| Task 6 on Resource 2 | 900 | 461 / 0.007 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58452 | 2749 / 42.95 | 2529 |
| timer | 56941 | 4507 / 70.42 | 2658 |
| intake | 11955 | 2190 / 34.22 | 1937 |
| lock | 77397 | 303 / 4.73 | 302 |
| unlock | 76497 | 1558 / 24.34 | 1358 |
| job_end | 55527 | 1801 / 28.14 | 1494 |
| now | 6224541 | 146 / 2.28 | 144 |
| raise | 6000 | 1742 / 27.22 | 1685 |
| dma | 8999 | 556 / 8.69 | 390 |
| epilogue | 58452 | 1118 / 17.47 | 463 |
| restart | 900 | 4507 / 70.42 | 4307 |
| enforce | 2925 | 4507 / 70.42 | 3336 |
| view | 72200 | 333 / 5.20 | 298 |

Entries that interrupted an executing Job: Task 0: 2956, Task 1: 11, Task 2: 29582, Task 3: 6186, Task 4: 1262, Task 5: 3125, Task 6: 2743

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 7209 cycles; Task 1: 2 (timer, intake), 4633 cycles; Task 2: 6 (dispatch, timer, intake), 15085 cycles; Task 3: 3 (dispatch, timer, intake), 7281 cycles; Task 4: 3 (dispatch, timer), 8308 cycles; Task 5: 4 (dispatch, timer, intake), 9829 cycles; Task 6: 4 (dispatch, timer, intake), 9829 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3229 cycles
- Task 0 at 179940.000 ms, 2848 cycles
- Task 2 at 179945.000 ms, 2840 cycles
- Task 0 at 179950.000 ms, 3333 cycles
- Task 5 at 179950.000 ms, 3333 cycles
- Task 0 at 179960.000 ms, 3084 cycles
- Task 3 at 179960.000 ms, 3084 cycles
- Task 2 at 179965.000 ms, 2840 cycles
- Task 0 at 179970.000 ms, 3057 cycles
- Task 5 at 179975.000 ms, 2852 cycles
- Task 0 at 179980.000 ms, 2832 cycles
- Task 0 at 179990.000 ms, 2764 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 6485242 of them, the longest 20451 cycles (319.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 6212833 |
| 512 to 1023 (8.0 to 16.0 µs) | 86190 |
| 1024 to 2047 (16.0 to 32.0 µs) | 71239 |
| 2048 to 4095 (32.0 to 64.0 µs) | 44597 |
| 4096 to 8191 (64.0 to 128.0 µs) | 58531 |
| 8192 to 16383 (128.0 to 256.0 µs) | 11803 |
| 16384 to 32767 (256.0 to 512.0 µs) | 49 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56600 | 20451 / 319.5 |
| timer | 25124 | 15970 / 249.5 |
| job_end | 6370264 | 3890 / 60.8 |
| epilogue | 33254 | 12133 / 189.6 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| none | 20285440 | 1 | 202854.400 |

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
loop:part0:sensor:hold=494
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=300
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
  detail=58073
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
  detail=288858
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
  detail=52207
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
  detail=288858
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
  detail=58069
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
  detail=52223
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
  detail=288906
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
  detail=58073
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
  detail=52183
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
  at_ms=179810
  detail=288743
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179810
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=58073
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
  detail=288858
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
  detail=52207
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
     Finished in 7.71s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

Not judged: the records are not this build's
