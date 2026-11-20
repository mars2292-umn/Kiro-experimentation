# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:970e7b7a8a16f4b95381bfdcd600a6287c4a437cc9d9bbd9d22190b0ecdba398

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8388 cycles (131.1 µs) against J_rel 12500 cycles |
| Kernel events | pass | 57 events mirrored (57 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 19, release_overlap of Task 2: 19, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 18164 cycles (283.8 µs) over 10605693 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10670 rises and 10000 response edges in the capture); latency 100 to 12250 µs, median 2240 µs, 0 later than 15 ms; Δ_stamp observed 4260 cycles (66.6 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.7 % of the run, the Kernel 58.3 % over 1724270 entries; runs per Task [18125, 5999, 31666, 5439, 10000, 7236, 1829] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21951 cycles (0.343 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21120 cycles (0.330 ms at 100000 samples/s; 18125 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 1032 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 2809 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 259 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8388 / 131.1 | 3764 | 2^12:16478 2^13:1520 2^14:1 |
| 2 | 8991 | 8388 / 131.1 | 3759 | 2^12:8077 2^13:913 2^14:1 |
| 3 | 4499 | 8388 / 131.1 | 3904 | 2^12:3587 2^13:911 2^14:1 |
| 5 | 7199 | 6887 / 107.6 | 3847 | 2^12:5664 2^13:1535 |
| 6 | 1799 | 5480 / 85.6 | 4205 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.944 | 0 | 0 | 0 |
| 1 | 5999 | 0.262 | 0 | 0 | 0 |
| 2 | 8992 | 54.488 | 0 | 0 | 0 |
| 3 | 4500 | 6.831 | 0 | 0 | 0 |
| 4 | 9999 | 12.275 | 10000 | 2316 | 2138 |
| 5 | 7200 | 11.410 | 0 | 0 | 0 |
| 6 | 1800 | 57.046 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 21951 / 0.343 (min 10207, mean 13145) | 0 above the Budget; 2^14:15656 2^15:2344 |
| Task 1 entry | 5999 | 7521 / 0.118 (min 3597, mean 3698) | 0 above the Budget; 2^12:5995 2^13:4 |
| Task 2 entry | 8992 | 2945444 / 46.023 (min 784234, mean 789737) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 202015 / 3.156 (min 186879, mean 195637) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 11856 / 0.185 (min 3633, mean 4056) | 0 above the Budget; 2^12:9230 2^13:767 2^14:2 |
| Task 5 entry | 7200 | 14577 / 0.228 (min 6939, mean 7306) | 0 above the Budget; 2^13:6852 2^14:348 |
| Task 6 entry | 1800 | 16839 / 0.263 (min 7975, mean 8302) | 0 above the Budget; 2^13:1678 2^14:121 2^15:1 |
| Task 0 on Resource 0 | 18000 | 4358 / 0.068 | |
| Task 0 on Resource 1 | 18000 | 4363 / 0.068 | |
| Task 2 on Resource 0 | 8992 | 773454 / 12.085 | |
| Task 2 on Resource 1 | 17984 | 4231 / 0.066 | |
| Task 5 on Resource 2 | 7200 | 5534 / 0.086 | |
| Task 6 on Resource 2 | 1800 | 7144 / 0.112 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 3318 / 51.84 | 3121 |
| timer | 50958 | 4500 / 70.31 | 3108 |
| intake | 10000 | 2447 / 38.23 | 2272 |
| lock | 71976 | 470 / 7.34 | 455 |
| unlock | 71976 | 1947 / 30.42 | 1718 |
| job_end | 56490 | 2214 / 34.59 | 1897 |
| now | 10373249 | 226 / 3.53 | 224 |
| raise | 6000 | 2239 / 34.98 | 2109 |
| dma | 1808 | 724 / 11.31 | 509 |
| epilogue | 56490 | 1253 / 19.58 | 759 |
| enforce | 8 | 3040 / 47.50 | 2986 |
| view | 80171 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 8685, Task 1: 4, Task 2: 53363, Task 3: 2632, Task 4: 771, Task 5: 409, Task 6: 152

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer), 9149 cycles; Task 1: 1 (timer), 3111 cycles; Task 2: 31 (dispatch, timer, intake), 95631 cycles; Task 3: 3 (dispatch, timer, intake), 8634 cycles; Task 4: 2 (dispatch, timer), 7296 cycles; Task 5: 2 (timer), 5930 cycles; Task 6: none

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3511 cycles
- Task 2 at 179940.000 ms, 3511 cycles
- Task 0 at 179950.000 ms, 4167 cycles
- Task 5 at 179950.000 ms, 4167 cycles
- Task 0 at 179960.000 ms, 3755 cycles
- Task 2 at 179960.000 ms, 3755 cycles
- Task 3 at 179960.000 ms, 3755 cycles
- Task 0 at 179970.000 ms, 3759 cycles
- Task 5 at 179975.000 ms, 3943 cycles
- Task 0 at 179980.000 ms, 3511 cycles
- Task 2 at 179980.000 ms, 3511 cycles
- Task 0 at 179990.000 ms, 3295 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 10605693 of them, the longest 18164 cycles (283.8 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 10349317 |
| 512 to 1023 (8.0 to 16.0 µs) | 73618 |
| 1024 to 2047 (16.0 to 32.0 µs) | 43498 |
| 2048 to 4095 (32.0 to 64.0 µs) | 65141 |
| 4096 to 8191 (64.0 to 128.0 µs) | 58616 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15502 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48381 | 18164 / 283.8 |
| timer | 44704 | 15326 / 239.5 |
| job_end | 10490113 | 4305 / 67.3 |
| epilogue | 22495 | 4086 / 63.8 |

## The Kernel's events

57 events mirrored (57 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 19, release_overlap of Task 2: 19, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 342017 | 18125 | 0.330 |
| 1 | 25822 | 5999 | 0.110 |
| 2 | 11207192 | 31666 | 12.300 |
| 3 | 1372969 | 5439 | 3.130 |
| 4 | 46900 | 10000 | 0.160 |
| 5 | 69948 | 7236 | 0.220 |
| 6 | 20218 | 1829 | 0.250 |
| none | 4912127 | 32804 | 7.190 |

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
blocked_releases=8995
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1032
loop:part0:sensor:late=2809
loop:part1:spin_ms:spin=0
loop:part1:log:busy=259
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
     Finished in 8.44s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json, docs/hil/kernel_timing_nrf52840_response.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3681805 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
