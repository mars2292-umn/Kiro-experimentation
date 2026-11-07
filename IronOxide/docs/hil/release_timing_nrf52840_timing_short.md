# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6043 cycles (94.4 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10634 rises and 10000 response edges in the capture); latency 90 to 12410 µs, median 2500 µs, 0 later than 15 ms; Δ_stamp observed 3656 cycles (57.1 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.3 % over 1998566 entries; runs per Task [18162, 6004, 31744, 5265, 9999, 7207, 1815] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 19919 cycles (0.311 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18162 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1384 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3778 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 347 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6043 / 94.4 | 3599 | 2^12:17081 2^13:918 |
| 2 | 8991 | 5331 / 83.3 | 3653 | 2^12:8083 2^13:908 |
| 3 | 4499 | 5331 / 83.3 | 3790 | 2^12:3591 2^13:908 |
| 5 | 7199 | 5204 / 81.3 | 3654 | 2^12:6294 2^13:905 |
| 6 | 1799 | 5204 / 81.3 | 4089 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.518 | 0 | 0 | 0 |
| 1 | 5999 | 0.238 | 0 | 0 | 0 |
| 2 | 8992 | 54.298 | 0 | 0 | 0 |
| 3 | 4500 | 6.618 | 0 | 0 | 0 |
| 4 | 9999 | 12.427 | 10000 | 2162 | 1971 |
| 5 | 7200 | 11.280 | 0 | 0 | 0 |
| 6 | 1800 | 43.579 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19919 / 0.311 (min 8839, mean 11482) | 0 above the Budget; 2^14:17962 2^15:38 |
| Task 1 entry | 5999 | 3187 / 0.050 (min 3007, mean 3099) | 0 above the Budget; 2^12:5999 |
| Task 2 entry | 8992 | 2950892 / 46.108 (min 781825, mean 786933) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200579 / 3.134 (min 181507, mean 195034) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10312 / 0.161 (min 3092, mean 3369) | 0 above the Budget; 2^12:9525 2^13:464 2^14:10 |
| Task 5 entry | 7200 | 13058 / 0.204 (min 5958, mean 6176) | 0 above the Budget; 2^13:7107 2^14:93 |
| Task 6 entry | 1800 | 25857 / 0.404 (min 6757, mean 9173) | 0 above the Budget; 2^13:574 2^14:1224 2^15:2 |
| Task 0 on Resource 0 | 18000 | 3928 / 0.061 | |
| Task 0 on Resource 1 | 18000 | 3933 / 0.061 | |
| Task 2 on Resource 0 | 8992 | 772927 / 12.077 | |
| Task 2 on Resource 1 | 17984 | 3770 / 0.059 | |
| Task 5 on Resource 2 | 7200 | 4744 / 0.074 | |
| Task 6 on Resource 2 | 1800 | 4660 / 0.073 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2905 / 45.39 | 2747 |
| timer | 52109 | 4392 / 68.62 | 3048 |
| intake | 10000 | 2293 / 35.83 | 2100 |
| lock | 71976 | 460 / 7.19 | 458 |
| unlock | 71976 | 1834 / 28.66 | 1613 |
| job_end | 56490 | 2081 / 32.52 | 1792 |
| now | 13915841 | 242 / 3.78 | 240 |
| raise | 6000 | 2074 / 32.41 | 1961 |
| dma | 1808 | 708 / 11.06 | 505 |
| epilogue | 56490 | 1217 / 19.02 | 722 |
| enforce | 8 | 2976 / 46.50 | 2933 |
| view | 80033 | 330 / 5.16 | 293 |

Entries that interrupted an executing Job: Task 0: 8872, Task 2: 53602, Task 3: 2473, Task 4: 484, Task 5: 126, Task 6: 1250

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3415 cycles
- Task 2 at 179940.000 ms, 3415 cycles
- Task 0 at 179950.000 ms, 3755 cycles
- Task 5 at 179950.000 ms, 3755 cycles
- Task 0 at 179960.000 ms, 3647 cycles
- Task 2 at 179960.000 ms, 3647 cycles
- Task 3 at 179960.000 ms, 3647 cycles
- Task 0 at 179970.000 ms, 3283 cycles
- Task 5 at 179975.000 ms, 3739 cycles
- Task 0 at 179980.000 ms, 3415 cycles
- Task 2 at 179980.000 ms, 3415 cycles
- Task 0 at 179990.000 ms, 3651 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 302956 | 18162 | 0.300 |
| 1 | 22899 | 6004 | 0.050 |
| 2 | 11153283 | 31744 | 12.220 |
| 3 | 1369186 | 5265 | 3.120 |
| 4 | 40262 | 9999 | 0.150 |
| 5 | 60087 | 7207 | 0.190 |
| 6 | 23368 | 1815 | 0.250 |
| none | 5025138 | 32942 | 7.290 |

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
loop:part0:sensor:late=3778
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
     Finished in 7.74s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3688615 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
