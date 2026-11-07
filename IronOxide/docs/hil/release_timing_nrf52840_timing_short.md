# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.0 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6620 cycles (103.4 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 actions recorded (0 held): 0 deferral(s) of a dispatch vector taken inside another stub's return window |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10619 rises and 10000 response edges in the capture); latency 90 to 12570 µs, median 2290 µs, 0 later than 15 ms; Δ_stamp observed 4156 cycles (64.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.2 % of the run, the Kernel 58.3 % over 1703808 entries; runs per Task [18157, 6003, 31984, 5504, 9998, 7239, 1802] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22271 cycles (0.348 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s; 18157 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1355 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3696 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 340 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6620 / 103.4 | 3580 | 2^12:16956 2^13:1043 |
| 2 | 8991 | 6620 / 103.4 | 3611 | 2^12:8028 2^13:963 |
| 3 | 4499 | 6620 / 103.4 | 3742 | 2^12:3566 2^13:933 |
| 5 | 7199 | 6620 / 103.4 | 3636 | 2^12:6233 2^13:966 |
| 6 | 1799 | 6620 / 103.4 | 4033 | 2^12:889 2^13:910 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.606 | 0 | 0 | 0 |
| 1 | 5999 | 0.242 | 0 | 0 | 0 |
| 2 | 8992 | 54.314 | 0 | 0 | 0 |
| 3 | 4500 | 6.793 | 0 | 0 | 0 |
| 4 | 9999 | 12.583 | 10000 | 2187 | 2010 |
| 5 | 7200 | 11.303 | 0 | 0 | 0 |
| 6 | 1800 | 43.576 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22271 / 0.348 (min 9041, mean 11739) | 0 above the Budget; 2^14:17635 2^15:365 |
| Task 1 entry | 5999 | 6732 / 0.105 (min 3034, mean 3189) | 0 above the Budget; 2^12:5899 2^13:100 |
| Task 2 entry | 8992 | 2952263 / 46.129 (min 782138, mean 787447) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200511 / 3.133 (min 177661, mean 194801) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10254 / 0.160 (min 3092, mean 3428) | 0 above the Budget; 2^12:9683 2^13:95 2^14:221 |
| Task 5 entry | 7200 | 13252 / 0.207 (min 6070, mean 6404) | 0 above the Budget; 2^13:7004 2^14:196 |
| Task 6 entry | 1800 | 25692 / 0.401 (min 6886, mean 9269) | 0 above the Budget; 2^13:561 2^14:1237 2^15:2 |
| Task 0 on Resource 0 | 18000 | 7228 / 0.113 | |
| Task 0 on Resource 1 | 18000 | 3869 / 0.060 | |
| Task 2 on Resource 0 | 8992 | 772837 / 12.076 | |
| Task 2 on Resource 1 | 17984 | 3704 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 7314 / 0.114 | |
| Task 6 on Resource 2 | 1800 | 3648 / 0.057 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2908 / 45.44 | 2745 |
| timer | 53175 | 4339 / 67.80 | 2996 |
| intake | 10000 | 2315 / 36.17 | 2137 |
| lock | 71976 | 466 / 7.28 | 464 |
| unlock | 71976 | 1913 / 29.89 | 1679 |
| job_end | 56490 | 2178 / 34.03 | 1873 |
| now | 13620241 | 244 / 3.81 | 241 |
| raise | 6000 | 2160 / 33.75 | 2044 |
| dma | 1808 | 709 / 11.08 | 506 |
| epilogue | 56490 | 1229 / 19.20 | 731 |
| enforce | 8 | 2987 / 46.67 | 2892 |
| view | 80531 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 9093, Task 1: 100, Task 2: 54482, Task 3: 2639, Task 4: 537, Task 5: 351, Task 6: 1244

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3360 cycles
- Task 2 at 179940.000 ms, 3360 cycles
- Task 0 at 179950.000 ms, 3872 cycles
- Task 5 at 179950.000 ms, 3872 cycles
- Task 0 at 179960.000 ms, 3580 cycles
- Task 2 at 179960.000 ms, 3580 cycles
- Task 3 at 179960.000 ms, 3580 cycles
- Task 0 at 179970.000 ms, 3600 cycles
- Task 5 at 179975.000 ms, 3412 cycles
- Task 0 at 179980.000 ms, 3360 cycles
- Task 2 at 179980.000 ms, 3360 cycles
- Task 0 at 179990.000 ms, 3496 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 311639 | 18157 | 0.340 |
| 1 | 24198 | 6003 | 0.100 |
| 2 | 11164233 | 31984 | 12.270 |
| 3 | 1369204 | 5504 | 3.110 |
| 4 | 41668 | 9998 | 0.150 |
| 5 | 63173 | 7239 | 0.200 |
| 6 | 23694 | 1802 | 0.250 |
| none | 4999234 | 32443 | 7.320 |

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
loop:part0:sensor:hold=1355
loop:part0:sensor:late=3696
loop:part1:spin_ms:spin=0
loop:part1:log:busy=340
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
     Finished in 7.37s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3690389 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
