# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7287 cycles (113.9 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10616 rises and 10000 response edges in the capture); latency 80 to 12550 µs, median 2310 µs, 0 later than 15 ms; Δ_stamp observed 5968 cycles (93.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.5 % over 1998405 entries; runs per Task [18324, 6000, 31970, 5006, 10202, 7263, 1801] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21945 cycles (0.343 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21120 cycles (0.330 ms at 100000 samples/s; 18324 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1384 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3775 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 347 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7287 / 113.9 | 3585 | 2^12:16809 2^13:1190 |
| 2 | 8991 | 7287 / 113.9 | 3623 | 2^12:7886 2^13:1105 |
| 3 | 4499 | 7287 / 113.9 | 3768 | 2^12:3500 2^13:999 |
| 5 | 7199 | 7287 / 113.9 | 3642 | 2^12:6222 2^13:977 |
| 6 | 1799 | 7287 / 113.9 | 4081 | 2^12:870 2^13:929 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.609 | 0 | 0 | 0 |
| 1 | 5999 | 0.275 | 0 | 0 | 0 |
| 2 | 8992 | 54.335 | 0 | 0 | 0 |
| 3 | 4500 | 6.846 | 0 | 0 | 0 |
| 4 | 9999 | 12.572 | 10000 | 2117 | 1935 |
| 5 | 7200 | 11.280 | 0 | 0 | 0 |
| 6 | 1800 | 40.028 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 21945 / 0.343 (min 8802, mean 11418) | 0 above the Budget; 2^14:17760 2^15:240 |
| Task 1 entry | 5999 | 6708 / 0.105 (min 3025, mean 3166) | 0 above the Budget; 2^12:5892 2^13:107 |
| Task 2 entry | 8992 | 2952586 / 46.134 (min 781984, mean 787104) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 199159 / 3.112 (min 183534, mean 195202) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12494 / 0.195 (min 3061, mean 3477) | 0 above the Budget; 2^12:9413 2^13:441 2^14:145 |
| Task 5 entry | 7200 | 12949 / 0.202 (min 5946, mean 6334) | 0 above the Budget; 2^13:6884 2^14:316 |
| Task 6 entry | 1800 | 16899 / 0.264 (min 6735, mean 8962) | 0 above the Budget; 2^13:643 2^14:1156 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7363 / 0.115 | |
| Task 0 on Resource 1 | 18000 | 3876 / 0.061 | |
| Task 2 on Resource 0 | 8992 | 772820 / 12.075 | |
| Task 2 on Resource 1 | 17984 | 3704 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 7125 / 0.111 | |
| Task 6 on Resource 2 | 1800 | 272 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2855 / 44.61 | 2690 |
| timer | 53170 | 4431 / 69.23 | 2978 |
| intake | 10000 | 2242 / 35.03 | 2060 |
| lock | 71976 | 464 / 7.25 | 463 |
| unlock | 71976 | 1779 / 27.80 | 1608 |
| job_end | 56490 | 2067 / 32.30 | 1799 |
| now | 13920695 | 242 / 3.78 | 240 |
| raise | 6000 | 2068 / 32.31 | 1976 |
| dma | 1808 | 712 / 11.12 | 498 |
| epilogue | 56490 | 1219 / 19.05 | 732 |
| enforce | 8 | 2911 / 45.48 | 2857 |
| view | 80241 | 328 / 5.12 | 290 |

Entries that interrupted an executing Job: Task 0: 8744, Task 1: 107, Task 2: 54690, Task 3: 2160, Task 4: 833, Task 5: 486, Task 6: 1158

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3324 cycles
- Task 2 at 179940.000 ms, 3324 cycles
- Task 0 at 179950.000 ms, 3487 cycles
- Task 5 at 179950.000 ms, 3487 cycles
- Task 0 at 179960.000 ms, 3572 cycles
- Task 2 at 179960.000 ms, 3572 cycles
- Task 3 at 179960.000 ms, 3572 cycles
- Task 0 at 179970.000 ms, 3575 cycles
- Task 5 at 179975.000 ms, 3287 cycles
- Task 0 at 179980.000 ms, 3324 cycles
- Task 2 at 179980.000 ms, 3324 cycles
- Task 0 at 179990.000 ms, 3211 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 301742 | 18324 | 0.330 |
| 1 | 23406 | 6000 | 0.100 |
| 2 | 11157720 | 31970 | 12.260 |
| 3 | 1369227 | 5006 | 3.100 |
| 4 | 42957 | 10202 | 0.150 |
| 5 | 62130 | 7263 | 0.190 |
| 6 | 22736 | 1801 | 0.250 |
| none | 5016815 | 32733 | 7.350 |

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
loop:part0:sensor:hold=1384
loop:part0:sensor:late=3775
loop:part1:spin_ms:spin=0
loop:part1:log:busy=347
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
     Finished in 7.36s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3690733 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
