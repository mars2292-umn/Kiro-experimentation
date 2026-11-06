# HIL release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | FAIL | the run ended without RSK-OK after 191 s (console attached) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6929 cycles (108.3 µs) against J_rel 10000 cycles |
| Kernel events | FAIL | 44 events mirrored (44 held, 0 of them the post-detach reboot's): usagefault of Task 0: 1, overrun of Task 2: 7, deadline_miss of Task 2: 14, release_overlap of Task 2: 14, mit_violation of Task 2: 7, debugger of Task none: 1; the trace froze at event 43 (usagefault of Task 0 at 148400.008 ms); unexpected: usagefault of Task 0 at 148400.008 ms (detail 0x0) |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 43 and holds the entries leading to it |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10607 rises and 10000 response edges in the capture); latency 80 to 12310 µs, median 2580 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.2 % over 1911644 entries; runs per Task [18336, 6000, 31772, 5003, 10213, 7270, 1820] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 18441 cycles (0.288 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 17920 cycles (0.280 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6929 / 108.3 | 3560 | 2^12:16846 2^13:1153 |
| 2 | 8991 | 6929 / 108.3 | 3623 | 2^12:7924 2^13:1067 |
| 3 | 4499 | 6929 / 108.3 | 3760 | 2^12:3538 2^13:961 |
| 5 | 7199 | 6929 / 108.3 | 3614 | 2^12:6255 2^13:944 |
| 6 | 1799 | 6929 / 108.3 | 4067 | 2^12:905 2^13:894 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 17999 | 9.576 | 0 | 0 | 0 |
| 1 | 5998 | 0.270 | 0 | 0 | 0 |
| 2 | 8991 | 54.258 | 0 | 0 | 0 |
| 3 | 4500 | 6.849 | 0 | 0 | 0 |
| 4 | 9999 | 12.330 | 10000 | 2083 | 1922 |
| 5 | 7200 | 11.264 | 0 | 0 | 0 |
| 6 | 1800 | 59.860 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 17999 | 18441 / 0.288 (min 8347, mean 11245) | 0 above the Budget; 2^14:17974 2^15:25 |
| Task 1 entry | 5998 | 6590 / 0.103 (min 2912, mean 3066) | 0 above the Budget; 2^12:5916 2^13:82 |
| Task 2 entry | 8991 | 2946065 / 46.032 (min 781549, mean 786692) | 8 above the Budget; 2^15:8991 |
| Task 3 entry | 4500 | 199256 / 3.113 (min 187244, mean 195138) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12207 / 0.191 (min 2946, mean 3523) | 0 above the Budget; 2^12:9061 2^13:768 2^14:170 |
| Task 5 entry | 7200 | 13084 / 0.204 (min 5790, mean 6225) | 0 above the Budget; 2^13:6912 2^14:288 |
| Task 6 entry | 1800 | 14284 / 0.223 (min 6674, mean 9008) | 0 above the Budget; 2^13:644 2^14:1156 |
| Task 0 on Resource 0 | 17999 | 3817 / 0.060 | |
| Task 0 on Resource 1 | 17999 | 3834 / 0.060 | |
| Task 2 on Resource 0 | 8991 | 772392 / 12.069 | |
| Task 2 on Resource 1 | 17982 | 3697 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 7039 / 0.110 | |
| Task 6 on Resource 2 | 1800 | 4494 / 0.070 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56489 | 2765 / 43.20 | 2605 |
| timer | 52702 | 4382 / 68.47 | 2967 |
| intake | 10000 | 2150 / 33.59 | 1989 |
| lock | 71971 | 450 / 7.03 | 444 |
| unlock | 71971 | 1778 / 27.78 | 1613 |
| job_end | 56487 | 2051 / 32.05 | 1799 |
| now | 13832414 | 238 / 3.72 | 238 |
| raise | 5999 | 2077 / 32.45 | 1965 |
| dma | 1800 | 723 / 11.30 | 718 |
| fault | 1 | 2508 / 39.19 | 2508 |
| epilogue | 56488 | 1213 / 18.95 | 714 |
| restart | 1 | 2508 / 39.19 | 2508 |
| enforce | 8 | 2910 / 45.47 | 2860 |
| view | 80079 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 8391, Task 1: 82, Task 2: 54014, Task 3: 2186, Task 4: 1215, Task 5: 456, Task 6: 1178

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3290 cycles
- Task 2 at 179940.000 ms, 3290 cycles
- Task 0 at 179950.000 ms, 3493 cycles
- Task 5 at 179950.000 ms, 3493 cycles
- Task 0 at 179960.000 ms, 3530 cycles
- Task 2 at 179960.000 ms, 3530 cycles
- Task 3 at 179960.000 ms, 3530 cycles
- Task 0 at 179970.000 ms, 3245 cycles
- Task 5 at 179975.000 ms, 3261 cycles
- Task 0 at 179980.000 ms, 3290 cycles
- Task 2 at 179980.000 ms, 3290 cycles
- Task 0 at 179990.000 ms, 3541 cycles

## The Kernel's events

44 events mirrored (44 held, 0 of them the post-detach reboot's): usagefault of Task 0: 1, overrun of Task 2: 7, deadline_miss of Task 2: 14, release_overlap of Task 2: 14, mit_violation of Task 2: 7, debugger of Task none: 1; the trace froze at event 43 (usagefault of Task 0 at 148400.008 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 43 | 148400.008 | usagefault | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 299063 | 18336 | 0.280 |
| 1 | 23268 | 6000 | 0.100 |
| 2 | 11147060 | 31772 | 12.210 |
| 3 | 1369224 | 5003 | 3.100 |
| 4 | 45238 | 10213 | 0.150 |
| 5 | 61934 | 7270 | 0.200 |
| 6 | 23195 | 1820 | 0.190 |
| none | 5027688 | 32891 | 7.820 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=3159
sensor=1578
log=4500
pulse=9999
echo=1052
blocked_releases=1578
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=4
  task=0
  at_ms=148400
  detail=0
  prs_lo=4194304
  prs_hi=0
event=16
  task=255
  at_ms=148400
  detail=4
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
expected_control=18000
expected_sensor=8992
expected_late_jobs=8
late_events_seen=28
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0xd46a inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:939:17
    Frame 1: report_and_exit @ 0xd44e
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/system/src/lib.rs:319:5
    Frame 2: safe_state @ 0xd9d4
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/decl/src/nrf52840_timing_short.rs:26:1
    Frame 3: irq_entry<rsk_system::Sys> @ 0xb44e inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:510:9
    Frame 4: rsk_irq_entry_rust @ 0xa74a
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-entry/src/lib.rs:154:16
    Frame 5: rsk_irq_entry @ 0x14c
    Frame 6: External interrupt #8 @ 0x871b04c7
    Frame 7: _p2_ebss @ 0x871b04c7
    Frame 8: _p2_ebss @ 0x39bd3ef6
     Finished in 7.34s
Error: Application exit
```
