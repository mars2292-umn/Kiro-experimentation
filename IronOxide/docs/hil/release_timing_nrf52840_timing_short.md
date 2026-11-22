# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:09a44a4f79210034d7734410b399d4c381d22c33ed361a388a1adc9a857593e9

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 193 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 241888 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (19 below 4 ticks, 32 below 8, 51 below 16); 10660 pended by software, the instant reached |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8337 cycles (130.3 µs) against J_rel 12500 cycles |
| Kernel events | pass | 59 events mirrored (59 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 20, release_overlap of Task 2: 20, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 18127 cycles (283.2 µs) over 8484636 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 6430 cycles (100.5 µs) over 8573784 holds, closed by a timer entry; against the record's L_kernel 11000 cycles |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10685 rises and 10001 response edges in the capture); latency 100 to 12520 µs, median 2280 µs, 0 later than 15 ms; Δ_stamp observed 6188 cycles (96.7 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 73.0 % of the run, the Kernel 61.3 % over 2478301 entries; runs per Task [18169, 6000, 31947, 5004, 10165, 7283, 1803] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23724 cycles (0.371 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23040 cycles (0.360 ms at 100000 samples/s; 18169 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 819 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 2224 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 206 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8337 / 130.3 | 3603 | 2^12:17195 2^13:803 2^14:1 |
| 2 | 8991 | 8337 / 130.3 | 3567 | 2^12:8630 2^13:360 2^14:1 |
| 3 | 4499 | 8337 / 130.3 | 3705 | 2^12:4220 2^13:278 2^14:1 |
| 5 | 7199 | 7776 / 121.5 | 3682 | 2^12:6593 2^13:606 |
| 6 | 1799 | 7776 / 121.5 | 4002 | 2^12:1576 2^13:223 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 10.031 | 0 | 0 | 0 |
| 1 | 5999 | 0.610 | 0 | 0 | 0 |
| 2 | 8992 | 54.571 | 0 | 0 | 0 |
| 3 | 4500 | 6.980 | 0 | 0 | 0 |
| 4 | 10000 | 12.540 | 10000 | 2194 | 2007 |
| 5 | 7200 | 11.573 | 0 | 0 | 0 |
| 6 | 1800 | 57.222 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 23724 / 0.371 (min 10672, mean 13482) | 0 above the Budget; 2^14:14950 2^15:3050 |
| Task 1 entry | 5999 | 15526 / 0.243 (min 3612, mean 3765) | 0 above the Budget; 2^12:5861 2^13:137 2^14:1 |
| Task 2 entry | 8992 | 2938662 / 45.917 (min 785403, mean 791420) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201041 / 3.141 (min 180490, mean 196814) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 14174 / 0.221 (min 3638, mean 4170) | 0 above the Budget; 2^12:9011 2^13:906 2^14:83 |
| Task 5 entry | 7200 | 12781 / 0.200 (min 7209, mean 7633) | 0 above the Budget; 2^13:6671 2^14:529 |
| Task 6 entry | 1800 | 23739 / 0.371 (min 8426, mean 8559) | 0 above the Budget; 2^14:1798 2^15:2 |
| Task 0 on Resource 0 | 18000 | 4358 / 0.068 | |
| Task 0 on Resource 1 | 18000 | 4379 / 0.068 | |
| Task 2 on Resource 0 | 8992 | 773955 / 12.093 | |
| Task 2 on Resource 1 | 17984 | 4342 / 0.068 | |
| Task 5 on Resource 2 | 7200 | 6039 / 0.094 | |
| Task 6 on Resource 2 | 1800 | 4170 / 0.065 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 3094 / 48.34 | 2901 |
| timer | 51589 | 4226 / 66.03 | 2850 |
| intake | 10000 | 2338 / 36.53 | 2154 |
| lock | 71976 | 458 / 7.16 | 456 |
| unlock | 71976 | 1800 / 28.12 | 1557 |
| job_end | 56491 | 2074 / 32.41 | 1754 |
| now | 8247454 | 227 / 3.55 | 227 |
| raise | 6000 | 2022 / 31.59 | 1900 |
| dma | 1808 | 725 / 11.33 | 502 |
| epilogue | 56491 | 1261 / 19.70 | 767 |
| enforce | 8 | 2826 / 44.16 | 2770 |
| view | 80201 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 8743, Task 1: 140, Task 2: 54044, Task 3: 2192, Task 4: 1155, Task 5: 611, Task 6: 43

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 7642 cycles; Task 1: 3 (dispatch, timer), 9730 cycles; Task 2: 35 (dispatch, timer, intake), 98698 cycles; Task 3: 1 (timer), 2674 cycles; Task 4: 3 (dispatch, timer), 9994 cycles; Task 5: 2 (dispatch, intake), 5256 cycles; Task 6: 5 (dispatch, timer), 15242 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3279 cycles
- Task 2 at 179940.000 ms, 3279 cycles
- Task 0 at 179950.000 ms, 4032 cycles
- Task 5 at 179950.000 ms, 4032 cycles
- Task 0 at 179960.000 ms, 3523 cycles
- Task 2 at 179960.000 ms, 3523 cycles
- Task 3 at 179960.000 ms, 3523 cycles
- Task 0 at 179970.000 ms, 3404 cycles
- Task 5 at 179975.000 ms, 3440 cycles
- Task 0 at 179980.000 ms, 3271 cycles
- Task 2 at 179980.000 ms, 3271 cycles
- Task 0 at 179990.000 ms, 3284 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 8484636 of them, the longest 18127 cycles (283.2 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 8301478 |
| 1024 to 2047 (16.0 to 32.0 µs) | 60739 |
| 2048 to 4095 (32.0 to 64.0 µs) | 51692 |
| 4096 to 8191 (64.0 to 128.0 µs) | 48619 |
| 8192 to 16383 (128.0 to 256.0 µs) | 22099 |
| 16384 to 32767 (256.0 to 512.0 µs) | 9 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 47647 | 18127 / 283.2 |
| timer | 46140 | 13787 / 215.4 |
| job_end | 8369662 | 5271 / 82.4 |
| epilogue | 21187 | 4186 / 65.4 |

The longest one's 5 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | job_end, an unwind follows, exception 37 pending | 0 | 2645 | 2645 / 41.3 | 0 / 0.0 |
| 2 | epilogue, exception 37 pending | 2835 | 4361 | 1526 / 23.8 | 190 / 3.0 |
| 3 | dispatch, started a Job, exception 24 pending | 4648 | 8178 | 3530 / 55.2 | 287 / 4.5 |
| 4 | timer, exception 36 pending | 8463 | 14232 | 5769 / 90.1 | 285 / 4.5 |
| 5 | dispatch, started a Job | 14534 | 18127 | 3593 / 56.1 | 302 / 4.7 |

Rust halves 17063 cycles (266.6 µs), between them 1064 cycles (16.6 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 8573784 of them, the longest 6430 cycles (100.5 µs), closed by a timer entry:

The longest was one entry: timer, exception 36 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3706 / 57.9 |
| timer | 6430 / 100.5 |
| job_end | 2339 / 36.5 |
| epilogue | 4441 / 69.4 |

## The Kernel's events

59 events mirrored (59 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 20, release_overlap of Task 2: 20, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 355149 | 18169 | 0.360 |
| 1 | 27164 | 6000 | 0.160 |
| 2 | 11233052 | 31947 | 12.260 |
| 3 | 1379370 | 5004 | 3.130 |
| 4 | 50880 | 10165 | 0.170 |
| 5 | 74395 | 7283 | 0.170 |
| 6 | 20935 | 1803 | 0.280 |
| none | 4858028 | 32778 | 7.190 |

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
pulse=10000
echo=5999
blocked_releases=8995
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=819
loop:part0:sensor:late=2224
loop:part1:spin_ms:spin=0
loop:part1:log:busy=206
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
overload_misses=0
RSK-OK
Firmware exited successfully
     Finished in 8.78s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.echo` (19408 cycles) exceeds its Budget (19200 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
