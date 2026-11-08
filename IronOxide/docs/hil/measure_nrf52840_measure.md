# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.3 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 193 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5897 cycles (92.1 µs) against J_rel 12500 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16398 cycles (256.2 µs) over 6501608 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10708 rises and 10001 response edges in the capture); latency 90 to 6510 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 4064 cycles (63.5 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 44.0 % of the run, the Kernel 40.3 % over 662323 entries; runs per Task [18042, 6000, 20997, 5459, 9999, 7234, 1801] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 29856 cycles (0.467 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 28800 cycles (0.450 ms at 100000 samples/s; 18042 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:373 | 10_000_000 | 546 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:411 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:373 | 10_000_000 | 275 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5806 / 90.7 | 3668 | 2^12:17292 2^13:707 |
| 2 | 8999 | 5897 / 92.1 | 3288 | 2^12:8996 2^13:3 |
| 3 | 4499 | 4369 / 68.3 | 3614 | 2^12:4399 2^13:100 |
| 5 | 7199 | 4262 / 66.6 | 3684 | 2^12:6498 2^13:701 |
| 6 | 1799 | 4125 / 64.5 | 3922 | 2^12:1700 2^13:99 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.728 | 0 | 0 | 0 |
| 1 | 5999 | 0.243 | 0 | 0 | 0 |
| 2 | 8999 | 6.944 | 0 | 0 | 0 |
| 3 | 4500 | 4.206 | 0 | 0 | 0 |
| 4 | 10000 | 6.530 | 10000 | 2174 | 2018 |
| 5 | 7200 | 7.338 | 0 | 0 | 0 |
| 6 | 1800 | 4.552 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 29856 / 0.467 (min 15972, mean 20089) | 0 above the Budget; 2^14:2266 2^15:15734 |
| Task 1 entry | 5999 | 6897 / 0.108 (min 3177, mean 3267) | 0 above the Budget; 2^12:5994 2^13:5 |
| Task 2 entry | 8999 | 407193 / 6.362 (min 399381, mean 401105) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 218997 / 3.422 (min 202193, mean 210119) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 10462 / 0.163 (min 3230, mean 3281) | 0 above the Budget; 2^12:9964 2^13:35 2^14:1 |
| Task 5 entry | 7200 | 21863 / 0.342 (min 8880, mean 11948) | 0 above the Budget; 2^14:6264 2^15:936 |
| Task 6 entry | 1800 | 19013 / 0.297 (min 7409, mean 10076) | 0 above the Budget; 2^13:493 2^14:1306 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7745 / 0.121 | |
| Task 0 on Resource 1 | 18000 | 4255 / 0.066 | |
| Task 2 on Resource 0 | 8999 | 389238 / 6.082 | |
| Task 2 on Resource 1 | 17998 | 3944 / 0.062 | |
| Task 5 on Resource 2 | 7200 | 4185 / 0.065 | |
| Task 6 on Resource 2 | 1800 | 3968 / 0.062 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56498 | 2882 / 45.03 | 2735 |
| timer | 56133 | 3886 / 60.72 | 3030 |
| intake | 10000 | 2299 / 35.92 | 2143 |
| lock | 71997 | 467 / 7.30 | 465 |
| unlock | 71997 | 1854 / 28.97 | 1653 |
| job_end | 56498 | 2006 / 31.34 | 1778 |
| now | 6123590 | 241 / 3.77 | 240 |
| raise | 6000 | 2045 / 31.95 | 1992 |
| dma | 1808 | 717 / 11.20 | 508 |
| epilogue | 56498 | 1223 / 19.11 | 583 |
| view | 69486 | 329 / 5.14 | 294 |
| send | 44552 | 2341 / 36.58 | 2181 |
| recv | 90671 | 870 / 13.59 | 772 |

Entries that interrupted an executing Job: Task 0: 11355, Task 1: 5, Task 2: 29439, Task 3: 3032, Task 4: 37, Task 5: 163, Task 6: 1310

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3914 cycles
- Task 0 at 179940.000 ms, 3233 cycles
- Task 2 at 179945.000 ms, 3289 cycles
- Task 0 at 179950.000 ms, 4190 cycles
- Task 5 at 179950.000 ms, 4190 cycles
- Task 0 at 179960.000 ms, 3469 cycles
- Task 3 at 179960.000 ms, 3469 cycles
- Task 2 at 179965.000 ms, 3289 cycles
- Task 0 at 179970.000 ms, 3918 cycles
- Task 5 at 179975.000 ms, 3229 cycles
- Task 0 at 179980.000 ms, 3233 cycles
- Task 0 at 179990.000 ms, 3165 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 6501608 of them, the longest 16398 cycles (256.2 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 6108884 |
| 512 to 1023 (8.0 to 16.0 µs) | 119458 |
| 1024 to 2047 (16.0 to 32.0 µs) | 99400 |
| 2048 to 4095 (32.0 to 64.0 µs) | 102596 |
| 4096 to 8191 (64.0 to 128.0 µs) | 58562 |
| 8192 to 16383 (128.0 to 256.0 µs) | 12707 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 47133 | 16398 / 256.2 |
| timer | 38519 | 13631 / 213.0 |
| job_end | 6383381 | 5375 / 84.0 |
| epilogue | 32575 | 7344 / 114.8 |

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 542775 | 18042 | 0.450 |
| 1 | 23063 | 6000 | 0.100 |
| 2 | 5688414 | 20997 | 6.230 |
| 3 | 1475573 | 5459 | 3.390 |
| 4 | 37342 | 9999 | 0.160 |
| 5 | 124353 | 7234 | 0.320 |
| 6 | 25742 | 1801 | 0.290 |
| none | 10080831 | 43506 | 8.340 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-mp/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-mp/IronOxide/target/xtask-hil/measure/nrf52840_measure/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-mp/IronOxide/target/xtask-hil/measure/nrf52840_measure/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=4500
pulse=10000
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
loop:part0:sensor:hold=546
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=275
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
     Finished in 7.79s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

Not judged: no Build_Manifest of the job (run `cargo xtask verify` first)
