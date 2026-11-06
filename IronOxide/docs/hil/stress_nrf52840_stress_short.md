# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 204 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8457 cycles (132.1 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.613 ms (103240 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.272 ms (17432 cycles) against WCRT 11.314 ms (724100 cycles); part0.sensor: 8999 Jobs, response at most 6.827 ms (436944 cycles) against WCRT 11.463 ms (733600 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11952 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2169 cycles (33.9 µs), mean 1970 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2044 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11951 response edges inside the run (target counted 11951) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.1 % of the run, the Kernel 45.5 % over 1376236 entries; runs per Task [18105, 6012, 20999, 7686, 12008, 7439, 1872] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22651 cycles (0.354 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 699 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 352 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8345 / 130.4 | 3357 | 2^12:17877 2^13:118 2^14:4 |
| 2 | 8999 | 8457 / 132.1 | 3197 | 2^12:8927 2^13:65 2^14:7 |
| 3 | 4499 | 8345 / 130.4 | 3447 | 2^12:4460 2^13:36 2^14:3 |
| 5 | 7199 | 8457 / 132.1 | 3435 | 2^12:7183 2^13:9 2^14:7 |
| 6 | 1799 | 6109 / 95.5 | 3721 | 2^12:1795 2^13:4 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.613 | 0 | 0 | 0 |
| 1 | 5999 | 0.272 | 0 | 0 | 0 |
| 2 | 8999 | 6.827 | 0 | 0 | 0 |
| 3 | 3375 | 4.237 | 0 | 0 | 0 |
| 4 | 11951 | 6.864 | 11952 | 2169 | 1970 |
| 5 | 6300 | 7.467 | 0 | 0 | 0 |
| 6 | 900 | 14.047 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22651 / 0.354 (min 8764, mean 11943) | 0 above the Budget; 2^14:17779 2^15:221 |
| Task 1 entry | 5999 | 9323 / 0.146 (min 3027, mean 3144) | 0 above the Budget; 2^12:5970 2^13:1 2^14:28 |
| Task 2 entry | 8999 | 408016 / 6.375 (min 398008, mean 399649) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204057 / 3.188 (min 190845, mean 195455) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11951 | 11806 / 0.184 (min 3089, mean 3206) | 0 above the Budget; 2^12:11740 2^13:157 2^14:54 |
| Task 5 entry | 6300 | 34043 / 0.532 (min 20126, mean 26294) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 29097 / 0.455 (min 16953, mean 25018) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 9705 / 0.152 | |
| Task 0 on Resource 1 | 18000 | 9750 / 0.152 | |
| Task 2 on Resource 0 | 8999 | 390945 / 6.109 | |
| Task 2 on Resource 1 | 17998 | 6297 / 0.098 | |
| Task 5 on Resource 2 | 12600 | 24850 / 0.388 | |
| Task 6 on Resource 2 | 918 | 271 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58449 | 2866 / 44.78 | 2681 |
| timer | 65419 | 4705 / 73.52 | 2916 |
| intake | 11952 | 2241 / 35.02 | 2041 |
| lock | 77415 | 458 / 7.16 | 456 |
| unlock | 76515 | 1828 / 28.56 | 1642 |
| job_end | 55524 | 2048 / 32.00 | 1794 |
| now | 8500323 | 237 / 3.70 | 236 |
| raise | 6000 | 2071 / 32.36 | 1993 |
| dma | 9328 | 709 / 11.08 | 541 |
| epilogue | 58449 | 1228 / 19.19 | 616 |
| restart | 900 | 4705 / 73.52 | 4565 |
| enforce | 2925 | 4705 / 73.52 | 3557 |
| view | 73996 | 328 / 5.12 | 294 |

Entries that interrupted an executing Job: Task 0: 11420, Task 1: 57, Task 2: 31812, Task 3: 9021, Task 4: 270, Task 5: 3192, Task 6: 2752

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3338 cycles
- Task 0 at 179940.000 ms, 3069 cycles
- Task 2 at 179945.000 ms, 3073 cycles
- Task 0 at 179950.000 ms, 3534 cycles
- Task 5 at 179950.000 ms, 3534 cycles
- Task 0 at 179960.000 ms, 3297 cycles
- Task 3 at 179960.000 ms, 3297 cycles
- Task 2 at 179965.000 ms, 3302 cycles
- Task 0 at 179970.000 ms, 3302 cycles
- Task 5 at 179975.000 ms, 3065 cycles
- Task 0 at 179980.000 ms, 3069 cycles
- Task 0 at 179990.000 ms, 2961 cycles

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 315636 | 18105 | 0.350 |
| 1 | 22898 | 6012 | 0.140 |
| 2 | 5667433 | 20999 | 6.260 |
| 3 | 1718997 | 7686 | 4.870 |
| 4 | 45407 | 12008 | 0.140 |
| 5 | 328233 | 7439 | 0.890 |
| 6 | 81439 | 1872 | 0.530 |
| none | 12242613 | 42903 | 16678.670 |

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
pulse=11951
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=699
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=352
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
  detail=32971
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
  at_ms=179333
  detail=385047
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179333
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=51761
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
  at_ms=179492
  detail=384573
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179492
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=32907
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
  detail=51761
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
  at_ms=179653
  detail=384527
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179653
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=32971
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
  at_ms=179776
  detail=52237
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179776
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179776
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179813
  detail=384689
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179813
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=32971
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
  at_ms=179972
  detail=384573
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179972
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=51761
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
     Finished in 7.39s
```
