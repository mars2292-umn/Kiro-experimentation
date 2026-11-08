# HIL release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36598 of 36599 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3671 s (operating duration 3660 s) |
| release timing (R9.10) | FAIL | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 10072 cycles (157.4 µs) against J_rel 10000 cycles |
| Kernel events | FAIL | 418 events mirrored (418 held, 0 of them the post-detach reboot's): overrun of Task 2: 68, overrun of Task 6: 1, deadline_miss of Task 2: 136, release_overlap of Task 2: 144, mit_violation of Task 2: 68, debugger of Task none: 1; the trace froze at event 417 (overrun of Task 6 at 1394043.913 ms); unexpected: overrun of Task 6 at 1394043.913 ms (detail 0x7f69) |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 5 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 417 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 2 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2 actions recorded (2 held, 2 of them deferrals: vectors 0x24, 0x28) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 23634 cycles (369.3 µs) over 229118044 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 203943 rises and 203334 response edges in the capture); latency 90 to 12610 µs, median 2450 µs, 0 later than 15 ms; Δ_stamp observed 6848 cycles (107.0 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.4 % of the run, the Kernel 63.3 % over 21282502 entries; runs per Task [369331, 122047, 647617, 107124, 204085, 147311, 36918] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22191 cycles (0.347 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s; 369331 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 1097 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 2992 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 275 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 10072 / 157.4 | 3681 | 2^12:338577 2^13:27421 2^14:1 |
| 2 | 182821 | 10072 / 157.4 | 3700 | 2^12:162817 2^13:20003 2^14:1 |
| 3 | 91499 | 10072 / 157.4 | 3846 | 2^12:72303 2^13:19195 2^14:1 |
| 5 | 146399 | 7700 / 120.3 | 3747 | 2^12:120764 2^13:25635 |
| 6 | 36599 | 7700 / 120.3 | 4153 | 2^12:18013 2^13:18586 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 9.885 | 0 | 0 | 0 |
| 1 | 121999 | 0.299 | 0 | 0 | 0 |
| 2 | 182822 | 54.443 | 0 | 0 | 0 |
| 3 | 91500 | 6.989 | 0 | 0 | 0 |
| 4 | 203333 | 12.616 | 203334 | 2201 | 1974 |
| 5 | 146400 | 11.423 | 0 | 0 | 0 |
| 6 | 36599 | 56.859 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 22191 / 0.347 (min 9439, mean 12270) | 0 above the Budget; 2^14:361125 2^15:4875 |
| Task 1 entry | 121999 | 9768 / 0.153 (min 3120, mean 3250) | 0 above the Budget; 2^12:121301 2^13:696 2^14:2 |
| Task 2 entry | 182822 | 2951796 / 46.122 (min 783149, mean 788712) | 178 above the Budget; 2^15:182822 |
| Task 3 entry | 91500 | 204369 / 3.193 (min 177693, mean 195178) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 21756 / 0.340 (min 3178, mean 3541) | 0 above the Budget; 2^12:194701 2^13:5491 2^14:3140 2^15:1 |
| Task 5 entry | 146400 | 18740 / 0.293 (min 6317, mean 6635) | 0 above the Budget; 2^13:142657 2^14:3708 2^15:35 |
| Task 6 entry | 36599 | 30921 / 0.483 (min 7248, mean 9736) | 0 above the Budget; 2^13:11984 2^14:24577 2^15:38 |
| Task 0 on Resource 0 | 366000 | 7914 / 0.124 | |
| Task 0 on Resource 1 | 366000 | 8051 / 0.126 | |
| Task 2 on Resource 0 | 182822 | 776225 / 12.129 | |
| Task 2 on Resource 1 | 365644 | 5931 / 0.093 | |
| Task 5 on Resource 2 | 146400 | 12635 / 0.197 | |
| Task 6 on Resource 2 | 36599 | 14868 / 0.232 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148660 | 2905 / 45.39 | 2724 |
| timer | 1068688 | 4512 / 70.50 | 3038 |
| intake | 203334 | 2327 / 36.36 | 2099 |
| lock | 1463465 | 488 / 7.62 | 461 |
| unlock | 1463465 | 1834 / 28.66 | 1612 |
| job_end | 1148653 | 2109 / 32.95 | 1795 |
| now | 224363234 | 239 / 3.73 | 236 |
| raise | 122000 | 2110 / 32.97 | 1972 |
| dma | 36607 | 734 / 11.47 | 499 |
| epilogue | 1148654 | 1208 / 18.88 | 708 |
| enforce | 179 | 3340 / 52.19 | 2916 |
| view | 1630987 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 181195, Task 1: 700, Task 2: 1098841, Task 3: 49348, Task 4: 12011, Task 5: 5625, Task 6: 25084

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3414 cycles
- Task 2 at 3659940.000 ms, 3414 cycles
- Task 0 at 3659950.000 ms, 3576 cycles
- Task 5 at 3659950.000 ms, 3576 cycles
- Task 0 at 3659960.000 ms, 3662 cycles
- Task 2 at 3659960.000 ms, 3662 cycles
- Task 3 at 3659960.000 ms, 3662 cycles
- Task 0 at 3659970.000 ms, 3404 cycles
- Task 5 at 3659975.000 ms, 3716 cycles
- Task 0 at 3659980.000 ms, 3474 cycles
- Task 2 at 3659980.000 ms, 3474 cycles
- Task 0 at 3659990.000 ms, 3312 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 229118044 of them, the longest 23634 cycles (369.3 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 223881308 |
| 512 to 1023 (8.0 to 16.0 µs) | 1487935 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1212470 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1105115 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1125140 |
| 8192 to 16383 (128.0 to 256.0 µs) | 306005 |
| 16384 to 32767 (256.0 to 512.0 µs) | 71 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 993800 | 23634 / 369.3 |
| timer | 940094 | 19607 / 306.4 |
| job_end | 226733326 | 5589 / 87.3 |
| epilogue | 450824 | 7105 / 111.0 |

## The Kernel's events

418 events mirrored (418 held, 0 of them the post-detach reboot's): overrun of Task 2: 68, overrun of Task 6: 1, deadline_miss of Task 2: 136, release_overlap of Task 2: 144, mit_violation of Task 2: 68, debugger of Task none: 1; the trace froze at event 417 (overrun of Task 6 at 1394043.913 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 417 | 1394043.913 | overrun | 6 | 2 | 0x7f69 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6592856 | 369331 | 0.340 |
| 1 | 481876 | 122047 | 0.150 |
| 2 | 227370599 | 647617 | 12.290 |
| 3 | 27861756 | 107124 | 3.120 |
| 4 | 859922 | 204085 | 0.260 |
| 5 | 1317488 | 147311 | 0.270 |
| 6 | 504188 | 36918 | 0.270 |
| none | 101007732 | 666317 | 7.320 |

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
blocked_releases=182830
filter=146400
filter_state_bits=1120403452
sampler=36599
dma_ok=36599
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1097
loop:part0:sensor:late=2992
loop:part1:spin_ms:spin=0
loop:part1:log:busy=275
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=36598
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
     Finished in 7.69s
```
