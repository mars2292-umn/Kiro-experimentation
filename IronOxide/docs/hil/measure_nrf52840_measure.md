# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 194 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7382 cycles (115.3 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16866 cycles (263.5 µs) over 6057043 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10753 rises and 10001 response edges in the capture); latency 90 to 6100 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 5560 cycles (86.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 44.3 % of the run, the Kernel 37.7 % over 1155981 entries; runs per Task [18035, 6005, 20995, 5137, 10117, 7372, 1820] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 29445 cycles (0.460 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 28800 cycles (0.450 ms at 100000 samples/s; 18035 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:373 | 10_000_000 | 505 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:411 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 255 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7382 / 115.3 | 3550 | 2^12:17273 2^13:726 |
| 2 | 8999 | 5801 / 90.6 | 3265 | 2^12:8940 2^13:59 |
| 3 | 4499 | 7382 / 115.3 | 3643 | 2^12:4365 2^13:134 |
| 5 | 7199 | 7382 / 115.3 | 3599 | 2^12:6719 2^13:480 |
| 6 | 1799 | 7382 / 115.3 | 3939 | 2^12:1739 2^13:60 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.736 | 0 | 0 | 0 |
| 1 | 5999 | 0.252 | 0 | 0 | 0 |
| 2 | 8999 | 7.003 | 0 | 0 | 0 |
| 3 | 4500 | 4.302 | 0 | 0 | 0 |
| 4 | 9999 | 6.128 | 10000 | 2227 | 2063 |
| 5 | 7200 | 7.440 | 0 | 0 | 0 |
| 6 | 1800 | 4.608 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 29445 / 0.460 (min 16709, mean 20769) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7071 / 0.110 (min 3319, mean 3391) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 409646 / 6.401 (min 400128, mean 401768) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 218929 / 3.421 (min 202877, mean 211622) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 14885 / 0.233 (min 3377, mean 3648) | 0 above the Budget; 2^12:9550 2^13:330 2^14:119 |
| Task 5 entry | 7200 | 26137 / 0.408 (min 9345, mean 12580) | 0 above the Budget; 2^14:5332 2^15:1868 |
| Task 6 entry | 1800 | 19973 / 0.312 (min 7733, mean 10358) | 0 above the Budget; 2^13:549 2^14:1249 2^15:2 |
| Task 0 on Resource 0 | 18000 | 4165 / 0.065 | |
| Task 0 on Resource 1 | 18000 | 4107 / 0.064 | |
| Task 2 on Resource 0 | 8999 | 389431 / 6.085 | |
| Task 2 on Resource 1 | 17998 | 4094 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 5468 / 0.085 | |
| Task 6 on Resource 2 | 1800 | 4193 / 0.066 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3056 / 47.75 | 2861 |
| timer | 56495 | 3907 / 61.05 | 2985 |
| intake | 10000 | 2338 / 36.53 | 2172 |
| lock | 71997 | 472 / 7.38 | 470 |
| unlock | 71997 | 1832 / 28.62 | 1641 |
| job_end | 56497 | 2061 / 32.20 | 1803 |
| now | 5674836 | 226 / 3.53 | 223 |
| raise | 6000 | 2069 / 32.33 | 1997 |
| dma | 1808 | 721 / 11.27 | 507 |
| epilogue | 56497 | 1261 / 19.70 | 616 |
| view | 69443 | 329 / 5.14 | 294 |
| send | 44552 | 2317 / 36.20 | 2158 |
| recv | 90671 | 901 / 14.08 | 805 |

Entries that interrupted an executing Job: Task 0: 11405, Task 1: 2, Task 2: 29070, Task 3: 2549, Task 4: 687, Task 5: 456, Task 6: 1322

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3870 cycles
- Task 0 at 179940.000 ms, 3197 cycles
- Task 2 at 179945.000 ms, 3257 cycles
- Task 0 at 179950.000 ms, 3626 cycles
- Task 5 at 179950.000 ms, 3626 cycles
- Task 0 at 179960.000 ms, 3441 cycles
- Task 3 at 179960.000 ms, 3441 cycles
- Task 2 at 179965.000 ms, 3201 cycles
- Task 0 at 179970.000 ms, 3354 cycles
- Task 5 at 179975.000 ms, 3197 cycles
- Task 0 at 179980.000 ms, 3257 cycles
- Task 0 at 179990.000 ms, 3125 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 6057043 of them, the longest 16866 cycles (263.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 5665170 |
| 512 to 1023 (8.0 to 16.0 µs) | 119185 |
| 1024 to 2047 (16.0 to 32.0 µs) | 101018 |
| 2048 to 4095 (32.0 to 64.0 µs) | 104690 |
| 4096 to 8191 (64.0 to 128.0 µs) | 53315 |
| 8192 to 16383 (128.0 to 256.0 µs) | 13644 |
| 16384 to 32767 (256.0 to 512.0 µs) | 21 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 46555 | 16866 / 263.5 |
| timer | 38257 | 16513 / 258.0 |
| job_end | 5939548 | 5565 / 87.0 |
| epilogue | 32683 | 7472 / 116.8 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 560467 | 18035 | 0.450 |
| 1 | 24039 | 6005 | 0.100 |
| 2 | 5698265 | 20995 | 6.300 |
| 3 | 1484023 | 5137 | 3.400 |
| 4 | 43132 | 10117 | 0.150 |
| 5 | 131683 | 7372 | 0.330 |
| 6 | 26479 | 1820 | 0.220 |
| none | 10028200 | 43545 | 8.480 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/measure/nrf52840_measure/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/measure/nrf52840_measure/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=4500
pulse=9999
echo=5999
blocked_releases=8999
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
telemetry_rx=17997
telemetry_last=17996
telemetry_full=0
pingpong_reps=4500
pingpong_last=17994
pingpong_reqs=4500
pingpong_rejected=0
pingpong_peer=0
telemetry_peer=0
protocol_violations=0
relay_reqs=2250
relay_rounds=2250
report_reqs=1125
report_rounds=1125
triangle_fwd=3600
triangle_acks=3600
triangle_rounds=3600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=505
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=255
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
events=3
event=23
  task=255
  at_ms=0
  detail=1
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
RSK-OK
Firmware exited successfully
     Finished in 8.44s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_measure.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged: the Analyzer rejected the inputs: [R28.4] the WCET of Task `part0.sensor` (512058 cycles) exceeds its Budget (512000 cycles)
