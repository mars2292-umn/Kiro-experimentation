# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 3671 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7380 cycles (115.3 µs) against J_rel 10000 cycles |
| Kernel events | pass | 1073 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 2: 84, deadline_miss of Task 2: 169, release_overlap of Task 2: 170, mit_violation of Task 2: 85, operating_end of Task none: 1, safe_state of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 actions recorded (0 held): 0 deferral(s) of a dispatch vector taken inside another stub's return window |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 203950 rises and 203334 response edges in the capture); latency 90 to 12630 µs, median 2400 µs, 0 later than 15 ms; Δ_stamp observed 6544 cycles (102.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.2 % of the run, the Kernel 58.4 % over 34653810 entries; runs per Task [369301, 122033, 647374, 107417, 204068, 147270, 36858] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22339 cycles (0.349 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s; 369301 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1355 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3702 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 340 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 7380 / 115.3 | 3574 | 2^12:345200 2^13:20799 |
| 2 | 182821 | 7380 / 115.3 | 3610 | 2^12:162982 2^13:19839 |
| 3 | 91499 | 7380 / 115.3 | 3741 | 2^12:72401 2^13:19098 |
| 5 | 146399 | 7380 / 115.3 | 3635 | 2^12:127226 2^13:19173 |
| 6 | 36599 | 7380 / 115.3 | 4033 | 2^12:18022 2^13:18577 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 9.698 | 0 | 0 | 0 |
| 1 | 121999 | 0.288 | 0 | 0 | 0 |
| 2 | 182822 | 54.376 | 0 | 0 | 0 |
| 3 | 91500 | 6.927 | 0 | 0 | 0 |
| 4 | 203333 | 12.656 | 203334 | 2239 | 2006 |
| 5 | 146400 | 11.364 | 0 | 0 | 0 |
| 6 | 36600 | 43.718 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 22339 / 0.349 (min 9023, mean 11688) | 0 above the Budget; 2^14:363361 2^15:2639 |
| Task 1 entry | 121999 | 6836 / 0.107 (min 3034, mean 3162) | 0 above the Budget; 2^12:121224 2^13:775 |
| Task 2 entry | 182822 | 2952615 / 46.135 (min 782134, mean 787481) | 178 above the Budget; 2^15:182822 |
| Task 3 entry | 91500 | 200735 / 3.136 (min 176747, mean 195148) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 13542 / 0.212 (min 3092, mean 3440) | 0 above the Budget; 2^12:194464 2^13:5944 2^14:2925 |
| Task 5 entry | 146400 | 17798 / 0.278 (min 6070, mean 6388) | 0 above the Budget; 2^13:142513 2^14:3824 2^15:63 |
| Task 6 entry | 36600 | 25988 / 0.406 (min 6886, mean 9199) | 0 above the Budget; 2^13:12222 2^14:24354 2^15:24 |
| Task 0 on Resource 0 | 366000 | 7404 / 0.116 | |
| Task 0 on Resource 1 | 366000 | 7497 / 0.117 | |
| Task 2 on Resource 0 | 182822 | 775741 / 12.121 | |
| Task 2 on Resource 1 | 365644 | 3804 / 0.059 | |
| Task 5 on Resource 2 | 146400 | 11831 / 0.185 | |
| Task 6 on Resource 2 | 36600 | 7879 / 0.123 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148655 | 2956 / 46.19 | 2745 |
| timer | 1070157 | 4375 / 68.36 | 2997 |
| intake | 203334 | 2367 / 36.98 | 2133 |
| lock | 1463466 | 466 / 7.28 | 464 |
| unlock | 1463466 | 1913 / 29.89 | 1678 |
| job_end | 1148654 | 2178 / 34.03 | 1871 |
| now | 277001004 | 244 / 3.81 | 241 |
| raise | 122000 | 2160 / 33.75 | 2035 |
| dma | 36608 | 709 / 11.08 | 506 |
| epilogue | 1148654 | 1229 / 19.20 | 726 |
| enforce | 178 | 3027 / 47.30 | 2877 |
| view | 1630906 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 180155, Task 1: 775, Task 2: 1099561, Task 3: 49625, Task 4: 12025, Task 5: 6527, Task 6: 24709

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3360 cycles
- Task 2 at 3659940.000 ms, 3360 cycles
- Task 0 at 3659950.000 ms, 3456 cycles
- Task 5 at 3659950.000 ms, 3456 cycles
- Task 0 at 3659960.000 ms, 3584 cycles
- Task 2 at 3659960.000 ms, 3584 cycles
- Task 3 at 3659960.000 ms, 3584 cycles
- Task 0 at 3659970.000 ms, 3472 cycles
- Task 5 at 3659975.000 ms, 3364 cycles
- Task 0 at 3659980.000 ms, 3360 cycles
- Task 2 at 3659980.000 ms, 3360 cycles
- Task 0 at 3659990.000 ms, 3384 cycles

## The Kernel's events

1073 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 2: 84, deadline_miss of Task 2: 169, release_overlap of Task 2: 170, mit_violation of Task 2: 85, operating_end of Task none: 1, safe_state of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6306149 | 369301 | 0.340 |
| 1 | 483949 | 122033 | 0.100 |
| 2 | 227007551 | 647374 | 12.270 |
| 3 | 27873478 | 107417 | 3.120 |
| 4 | 855428 | 204068 | 0.200 |
| 5 | 1281356 | 147270 | 0.260 |
| 6 | 478876 | 36858 | 0.250 |
| none | 101708726 | 666396 | 7.340 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182822
log=91500
pulse=203333
echo=121999
blocked_releases=182822
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1355
loop:part0:sensor:late=3702
loop:part1:spin_ms:spin=0
loop:part1:log:busy=340
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=36599
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
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=366000
expected_sensor=182822
expected_late_jobs=178
late_events_seen=30
RSK-OK
Firmware exited successfully
     Finished in 7.43s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3690769 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
