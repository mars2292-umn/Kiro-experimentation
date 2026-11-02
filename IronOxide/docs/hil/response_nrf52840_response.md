# HIL response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | the report ran after 22.7 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 29 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 253 ms: filter's fault \| end_job at 253 ms: END_JOB as part2's fault response \| overrun at 419 ms: log's Budget overrun \| end_job at 419 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| overrun at 1319 ms: sensor's Budget overrun, RECORD_AND_CONTINUE \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (KD-R9.6: the running Job's own, at 1,325 ms, is not reported), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 24 (about 25 since the restart; 47 had it kept its state), pulse 54 (about 56 since the restart) |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 44 answered pulses before part1's restart (of 43), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 99 matched in all, latency 70 to 8000 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.095 | overrun | 0 | 0 | 0xfd30 |
| 2 | 253.425 | memmanage | 5 | 2 | 0x0 |
| 3 | 253.425 | end_job | 5 | 2 | 0x2 |
| 4 | 419.695 | overrun | 3 | 1 | 0x5df4b |
| 5 | 419.695 | end_job | 3 | 1 | 0x6 |
| 6 | 501.191 | overrun | 5 | 2 | 0x7f34 |
| 7 | 501.191 | partition_restart | - | 2 | 0x6 |
| 8 | 501.191 | resource_released | 5 | 2 | 0x1 |
| 9 | 800.218 | memmanage | 3 | 1 | 0x0 |
| 10 | 800.218 | partition_restart | - | 1 | 0x2 |
| 11 | 1319.809 | overrun | 2 | 0 | 0x89acb |
| 12 | 1325.011 | release_overlap | 2 | 0 | 0x0 |
| 13 | 1328.678 | deadline_miss | 4 | 1 | 0x0 |
| 14 | 1345.012 | mit_violation | 2 | 0 | 0x0 |
| 15 | 1345.012 | deadline_miss | 2 | 0 | 0x0 |
| 16 | 1350.009 | mit_violation | 5 | 2 | 0x0 |
| 17 | 1350.009 | deadline_miss | 5 | 2 | 0x0 |
| 18 | 1350.009 | partition_stop | - | 2 | 0x7 |
| 19 | 1360.010 | mit_violation | 3 | 1 | 0x0 |
| 20 | 1360.010 | deadline_miss | 3 | 1 | 0x0 |
| 21 | 1365.008 | mit_violation | 2 | 0 | 0x0 |
| 22 | 1385.013 | mit_violation | 2 | 0 | 0x0 |
| 23 | 1400.008 | mit_violation | 3 | 1 | 0x0 |
| 24 | 1405.009 | mit_violation | 2 | 0 | 0x0 |
| 25 | 1425.010 | mit_violation | 2 | 0 | 0x0 |
| 26 | 1440.009 | mit_violation | 3 | 1 | 0x0 |
| 27 | 1900.008 | memmanage | 0 | 0 | 0x0 |
| 28 | 1900.008 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 3873 / 60.5 | 3079 | 2^12:190 |
| 2 | 90 | 3381 / 52.8 | 2882 | 2^12:90 |
| 3 | 44 | 3585 / 56.0 | 3164 | 2^12:44 |
| 5 | 53 | 3585 / 56.0 | 3154 | 2^12:53 |
| 6 | 13 | 3585 / 56.0 | 3439 | 2^12:13 |

Releases recorded: 390; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 3.458 | 0 | 0 | 0 |
| 1 | 63 | 0.209 | 0 | 0 | 0 |
| 2 | 90 | 128.631 | 0 | 0 | 0 |
| 3 | 43 | 133.551 | 0 | 0 | 0 |
| 4 | 98 | 128.538 | 98 | 1912 | 1771 |
| 5 | 51 | 8.907 | 0 | 0 | 0 |
| 6 | 13 | 19.874 | 0 | 0 | 0 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3081 cycles
- Task 3 at 1840.000 ms, 3081 cycles
- Task 2 at 1845.000 ms, 2781 cycles
- Task 0 at 1850.000 ms, 3081 cycles
- Task 0 at 1860.000 ms, 2825 cycles
- Task 2 at 1865.000 ms, 2825 cycles
- Task 0 at 1870.000 ms, 3201 cycles
- Task 0 at 1880.000 ms, 3037 cycles
- Task 3 at 1880.000 ms, 3037 cycles
- Task 2 at 1885.000 ms, 2825 cycles
- Task 0 at 1890.000 ms, 2981 cycles
- Task 0 at 1900.000 ms, 3145 cycles

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 92000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/response/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/response/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=190
sensor=90
log=24
pulse=54
echo=63
blocked_releases=90
filter=32
filter_state_bits=1120220744
sampler=8
dma_ok=8
first_memmanage_cfsr=2147483778
first_memmanage_mmfar=536870912
events=29
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=6
  task=0
  at_ms=201
  detail=64816
  prs_lo=134217728
  prs_hi=0
event=2
  task=5
  at_ms=253
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=5
  at_ms=253
  detail=2
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=419
  detail=384843
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=419
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=501
  detail=32564
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=501
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=501
  detail=1
  prs_lo=128
  prs_hi=0
event=2
  task=3
  at_ms=800
  detail=0
  prs_lo=541065216
  prs_hi=0
event=16
  task=255
  at_ms=800
  detail=2
  prs_lo=0
  prs_hi=0
event=6
  task=2
  at_ms=1319
  detail=563915
  prs_lo=134217728
  prs_hi=0
event=8
  task=2
  at_ms=1325
  detail=0
  prs_lo=4096
  prs_hi=0
event=7
  task=4
  at_ms=1328
  detail=0
  prs_lo=134217728
  prs_hi=0
event=9
  task=2
  at_ms=1345
  detail=0
  prs_lo=4096
  prs_hi=0
event=7
  task=2
  at_ms=1345
  detail=0
  prs_lo=134217728
  prs_hi=0
event=9
  task=5
  at_ms=1350
  detail=0
  prs_lo=4096
  prs_hi=0
event=7
  task=5
  at_ms=1350
  detail=0
  prs_lo=134217728
  prs_hi=0
event=17
  task=255
  at_ms=1350
  detail=7
  prs_lo=0
  prs_hi=0
event=9
  task=3
  at_ms=1360
  detail=0
  prs_lo=4096
  prs_hi=0
event=7
  task=3
  at_ms=1360
  detail=0
  prs_lo=134217728
  prs_hi=0
event=9
  task=2
  at_ms=1365
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=2
  at_ms=1385
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=3
  at_ms=1400
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=2
  at_ms=1405
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=2
  at_ms=1425
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=3
  at_ms=1440
  detail=0
  prs_lo=4096
  prs_hi=0
event=2
  task=0
  at_ms=1900
  detail=0
  prs_lo=541065216
  prs_hi=0
event=18
  task=255
  at_ms=1900
  detail=0
  prs_lo=0
  prs_hi=0
RSK-OK
Firmware exited successfully
     Finished in 6.63s
```
