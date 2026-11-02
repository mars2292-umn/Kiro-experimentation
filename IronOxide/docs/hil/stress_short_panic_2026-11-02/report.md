# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **FAIL**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | FAIL | the run ended without RSK-OK after 59 s (console attached) |
| release timing (R9.10) | pass | 8310 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7423 cycles (116.0 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 3695 Jobs, response at most 3.545 ms (226908 cycles) against WCRT 9.873 ms (631900 cycles); part0.echo: 1231 Jobs, response at most 0.305 ms (19512 cycles) against WCRT 11.195 ms (716500 cycles); part0.sensor: 1847 Jobs, response at most 8.789 ms (562484 cycles) against WCRT 11.339 ms (725700 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 2454 intakes against at most 12001 MIT windows of 15 ms (20.4 % of the windows); intake handler at most 2036 cycles (31.8 µs), mean 1857 cycles: the measured Δ_irq; processor time 0.0396 % of the run |
| Kernel events | FAIL | 1303 events mirrored (1024 held, 4 of them the post-detach reboot's): panic of Task none: 1, overrun of Task 3: 146, overrun of Task 5: 146, overrun of Task 6: 145, resource_released of Task 5: 144, partition_restart of Task none: 144, safe_state of Task none: 1, end_job of Task 3: 146, end_job of Task 5: 2, end_job of Task 6: 145; the trace froze at event 1297 (panic of Task 255 at 36940.406 ms); unexpected: panic of Task 255 at 36940.406 ms (detail 0x0) |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 1297 and holds the entries leading to it |
| responses under the storm | FAIL | 2454 response edges inside the run (target counted 2454) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 38.7 % of the run, the Kernel 67.9 % over 588774 entries; runs per Task [3706, 1231, 4310, 2533, 2474, 1514, 381] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 3694 | 6911 / 108.0 | 3162 | 2^12:3670 2^13:24 |
| 2 | 1847 | 7127 / 111.4 | 3028 | 2^12:1830 2^13:17 |
| 3 | 923 | 6911 / 108.0 | 3258 | 2^12:913 2^13:10 |
| 5 | 1477 | 7423 / 116.0 | 3293 | 2^12:1467 2^13:10 |
| 6 | 369 | 5495 / 85.9 | 3536 | 2^12:365 2^13:4 |

Releases recorded: 8310; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 3695 | 3.545 | 0 | 0 | 0 |
| 1 | 1231 | 0.305 | 0 | 0 | 0 |
| 2 | 1847 | 8.789 | 0 | 0 | 0 |
| 3 | 739 | 23.944 | 0 | 0 | 0 |
| 4 | 2454 | 8.862 | 2454 | 2036 | 1857 |
| 5 | 1292 | 9.384 | 0 | 0 | 0 |
| 6 | 185 | 4.312 | 0 | 0 | 0 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 36890.000 ms, 2999 cycles
- Task 0 at 36900.000 ms, 3399 cycles
- Task 5 at 36900.000 ms, 3399 cycles
- Task 6 at 36900.000 ms, 3399 cycles
- Task 2 at 36905.000 ms, 2883 cycles
- Task 0 at 36910.000 ms, 3051 cycles
- Task 0 at 36920.000 ms, 3103 cycles
- Task 3 at 36920.000 ms, 3103 cycles
- Task 2 at 36925.000 ms, 3651 cycles
- Task 5 at 36925.000 ms, 3651 cycles
- Task 0 at 36930.000 ms, 3127 cycles
- Task 0 at 36940.000 ms, 3255 cycles

## The Kernel's events

1303 events mirrored (1024 held, 4 of them the post-detach reboot's): panic of Task none: 1, overrun of Task 3: 146, overrun of Task 5: 146, overrun of Task 6: 145, resource_released of Task 5: 144, partition_restart of Task none: 144, safe_state of Task none: 1, end_job of Task 3: 146, end_job of Task 5: 2, end_job of Task 6: 145; the trace froze at event 1297 (panic of Task 255 at 36940.406 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 1297 | 36940.406 | panic | 255 | 255 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 56906 | 3706 | 0.300 |
| 1 | 4402 | 1231 | 0.180 |
| 2 | 1525219 | 4310 | 8.230 |
| 3 | 632387 | 2533 | 6.060 |
| 4 | 12905 | 2474 | 0.130 |
| 5 | 59317 | 1514 | 0.930 |
| 6 | 12262 | 381 | 0.960 |
| none | 3642970 | 7862 | 15632.710 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=3695
sensor=1847
log=739
pulse=2454
echo=1231
blocked_releases=1847
filter=6
filter_state_bits=1113357148
sampler=1
dma_ok=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=5
  at_ms=36175
  detail=32703
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=36175
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=36175
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=36299
  detail=384863
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=36299
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=36301
  detail=32427
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=36301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=36340
  detail=384509
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=36340
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=36375
  detail=32840
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=36375
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=36375
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=36501
  detail=32500
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=36501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=36540
  detail=384533
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=36540
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=36575
  detail=32492
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=36575
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=36575
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=36701
  detail=32852
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=36701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=36775
  detail=32703
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=36775
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=36775
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=36899
  detail=384863
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=36899
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=36901
  detail=32500
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=36901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=36940
  detail=384491
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=36940
  detail=6
  prs_lo=0
  prs_hi=0
event=1
  task=255
  at_ms=36940
  detail=0
  prs_lo=524288
  prs_hi=0
event=18
  task=255
  at_ms=36940
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=18000
expected_sensor=9000
part0_events=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0xb7cc inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:924:17
    Frame 1: report_and_exit @ 0xb7b4
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/system/src/lib.rs:249:5
    Frame 2: safe_state @ 0xbc8e
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/decl/src/nrf52840_stress_short.rs:13:1
    Frame 3: svc<rsk_system::Sys> @ 0xad00 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:406:9
    Frame 4: rsk_svc_rust @ 0xa920
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-entry/src/lib.rs:162:5
    Frame 5: rsk_svc_entry @ 0x1d6
    Frame 6: SVC @ 0xaf0b04c7
    Frame 7: _p2_ebss @ 0xaf0b04c7
    Frame 8: _p2_ebss @ 0x39b53ef6
     Finished in 6.71s
Error: Application exit
```
