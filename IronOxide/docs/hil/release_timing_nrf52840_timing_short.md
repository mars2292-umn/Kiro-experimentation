# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5940 cycles (92.8 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10621 rises and 10000 response edges in the capture); latency 80 to 12420 µs, median 2430 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.2 % over 1801727 entries; runs per Task [18322, 6000, 31771, 5119, 9998, 7256, 1825] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 18764 cycles (0.293 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 18560 cycles (0.290 ms at 100000 samples/s; 18322 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1364 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3718 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 342 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5940 / 92.8 | 3646 | 2^12:17075 2^13:924 |
| 2 | 8991 | 5689 / 88.9 | 3665 | 2^12:8079 2^13:912 |
| 3 | 4499 | 5689 / 88.9 | 3803 | 2^12:3590 2^13:909 |
| 5 | 7199 | 5097 / 79.6 | 3691 | 2^12:6295 2^13:904 |
| 6 | 1799 | 5097 / 79.6 | 4110 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.627 | 0 | 0 | 0 |
| 1 | 5999 | 0.277 | 0 | 0 | 0 |
| 2 | 8992 | 54.296 | 0 | 0 | 0 |
| 3 | 4500 | 6.876 | 0 | 0 | 0 |
| 4 | 9999 | 12.433 | 10000 | 2189 | 1994 |
| 5 | 7200 | 11.303 | 0 | 0 | 0 |
| 6 | 1800 | 43.578 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18764 / 0.293 (min 8884, mean 11484) | 0 above the Budget; 2^14:17993 2^15:7 |
| Task 1 entry | 5999 | 9257 / 0.145 (min 2943, mean 3095) | 0 above the Budget; 2^12:5886 2^13:112 2^14:1 |
| Task 2 entry | 8992 | 2951868 / 46.123 (min 782164, mean 787041) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200045 / 3.126 (min 187139, mean 195156) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10267 / 0.160 (min 3029, mean 3324) | 0 above the Budget; 2^12:9503 2^13:482 2^14:14 |
| Task 5 entry | 7200 | 16759 / 0.262 (min 5969, mean 6241) | 0 above the Budget; 2^13:7044 2^14:155 2^15:1 |
| Task 6 entry | 1800 | 25786 / 0.403 (min 6773, mean 9263) | 0 above the Budget; 2^13:532 2^14:1266 2^15:2 |
| Task 0 on Resource 0 | 18000 | 3915 / 0.061 | |
| Task 0 on Resource 1 | 18000 | 3916 / 0.061 | |
| Task 2 on Resource 0 | 8992 | 772803 / 12.075 | |
| Task 2 on Resource 1 | 17984 | 3856 / 0.060 | |
| Task 5 on Resource 2 | 7200 | 7405 / 0.116 | |
| Task 6 on Resource 2 | 1800 | 4580 / 0.072 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2810 / 43.91 | 2633 |
| timer | 52372 | 4462 / 69.72 | 3040 |
| intake | 10000 | 2256 / 35.25 | 2061 |
| lock | 71976 | 461 / 7.20 | 459 |
| unlock | 71976 | 1846 / 28.84 | 1664 |
| job_end | 56490 | 2114 / 33.03 | 1858 |
| now | 13719328 | 241 / 3.77 | 238 |
| raise | 6000 | 2129 / 33.27 | 2017 |
| dma | 1808 | 704 / 11.00 | 503 |
| epilogue | 56490 | 1224 / 19.12 | 728 |
| enforce | 8 | 2974 / 46.47 | 2926 |
| view | 79973 | 328 / 5.12 | 290 |

Entries that interrupted an executing Job: Task 0: 8487, Task 1: 114, Task 2: 54016, Task 3: 2320, Task 4: 510, Task 5: 262, Task 6: 1295

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3417 cycles
- Task 2 at 179940.000 ms, 3417 cycles
- Task 0 at 179950.000 ms, 3784 cycles
- Task 5 at 179950.000 ms, 3784 cycles
- Task 0 at 179960.000 ms, 3649 cycles
- Task 2 at 179960.000 ms, 3649 cycles
- Task 3 at 179960.000 ms, 3649 cycles
- Task 0 at 179970.000 ms, 3704 cycles
- Task 5 at 179975.000 ms, 3468 cycles
- Task 0 at 179980.000 ms, 3417 cycles
- Task 2 at 179980.000 ms, 3417 cycles
- Task 0 at 179990.000 ms, 3452 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 306991 | 18322 | 0.290 |
| 1 | 23982 | 6000 | 0.140 |
| 2 | 11154625 | 31771 | 12.220 |
| 3 | 1370256 | 5119 | 3.110 |
| 4 | 41515 | 9998 | 0.160 |
| 5 | 62365 | 7256 | 0.250 |
| 6 | 23992 | 1825 | 0.250 |
| none | 5012934 | 33001 | 7.380 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8992
log=4500
pulse=9999
echo=5999
blocked_releases=8992
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1364
loop:part0:sensor:late=3718
loop:part1:spin_ms:spin=0
loop:part1:log:busy=342
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=1799
dma_after_stop=0
spim0_enable=7
spim0_shorts=131072
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
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
late_events_seen=30
RSK-OK
Firmware exited successfully
     Finished in 7.39s
```
