# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.0 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6580 cycles (102.8 µs) against J_rel 12500 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 18, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17918 cycles (280.0 µs) over 9420485 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10669 rises and 10000 response edges in the capture); latency 90 to 12630 µs, median 2360 µs, 0 later than 15 ms; Δ_stamp observed 4212 cycles (65.8 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.6 % of the run, the Kernel 52.5 % over 2933525 entries; runs per Task [18175, 6001, 31972, 5408, 10000, 7243, 1803] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20070 cycles (0.314 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18175 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 914 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 2468 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 228 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6580 / 102.8 | 3660 | 2^12:17052 2^13:947 |
| 2 | 8991 | 6580 / 102.8 | 3719 | 2^12:8079 2^13:912 |
| 3 | 4499 | 6580 / 102.8 | 3857 | 2^12:3588 2^13:911 |
| 5 | 7199 | 5019 / 78.4 | 3735 | 2^12:6271 2^13:928 |
| 6 | 1799 | 5019 / 78.4 | 4163 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.851 | 0 | 0 | 0 |
| 1 | 5999 | 0.255 | 0 | 0 | 0 |
| 2 | 8992 | 54.439 | 0 | 0 | 0 |
| 3 | 4500 | 6.883 | 0 | 0 | 0 |
| 4 | 9999 | 12.639 | 10000 | 2214 | 2047 |
| 5 | 7200 | 11.418 | 0 | 0 | 0 |
| 6 | 1800 | 56.770 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20070 / 0.314 (min 9894, mean 12667) | 0 above the Budget; 2^14:17649 2^15:351 |
| Task 1 entry | 5999 | 7098 / 0.111 (min 3390, mean 3483) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8992 | 2949096 / 46.080 (min 783894, mean 789523) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201548 / 3.149 (min 180863, mean 195586) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 11663 / 0.182 (min 3522, mean 3750) | 0 above the Budget; 2^12:9780 2^13:218 2^14:1 |
| Task 5 entry | 7200 | 12009 / 0.188 (min 6772, mean 6990) | 0 above the Budget; 2^13:7048 2^14:152 |
| Task 6 entry | 1800 | 20719 / 0.324 (min 7962, mean 8168) | 0 above the Budget; 2^13:1728 2^14:70 2^15:2 |
| Task 0 on Resource 0 | 18000 | 4211 / 0.066 | |
| Task 0 on Resource 1 | 18000 | 4214 / 0.066 | |
| Task 2 on Resource 0 | 8992 | 775541 / 12.118 | |
| Task 2 on Resource 1 | 17984 | 4120 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 5521 / 0.086 | |
| Task 6 on Resource 2 | 1800 | 7124 / 0.111 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 3077 / 48.08 | 2895 |
| timer | 51090 | 4434 / 69.28 | 3051 |
| intake | 10000 | 2323 / 36.30 | 2155 |
| lock | 71976 | 457 / 7.14 | 441 |
| unlock | 71976 | 1825 / 28.52 | 1604 |
| job_end | 56490 | 2094 / 32.72 | 1800 |
| now | 9184472 | 221 / 3.45 | 218 |
| raise | 6000 | 2082 / 32.53 | 1960 |
| dma | 1808 | 827 / 12.92 | 495 |
| epilogue | 56490 | 1234 / 19.28 | 748 |
| enforce | 8 | 2982 / 46.59 | 2930 |
| view | 80431 | 308 / 4.81 | 273 |

Entries that interrupted an executing Job: Task 0: 8885, Task 1: 3, Task 2: 54147, Task 3: 2582, Task 4: 220, Task 5: 197, Task 6: 76

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3471 cycles
- Task 2 at 179940.000 ms, 3471 cycles
- Task 0 at 179950.000 ms, 3612 cycles
- Task 5 at 179950.000 ms, 3612 cycles
- Task 0 at 179960.000 ms, 3703 cycles
- Task 2 at 179960.000 ms, 3703 cycles
- Task 3 at 179960.000 ms, 3703 cycles
- Task 0 at 179970.000 ms, 3444 cycles
- Task 5 at 179975.000 ms, 3788 cycles
- Task 0 at 179980.000 ms, 3471 cycles
- Task 2 at 179980.000 ms, 3471 cycles
- Task 0 at 179990.000 ms, 3336 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 9420485 of them, the longest 17918 cycles (280.0 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 9163048 |
| 512 to 1023 (8.0 to 16.0 µs) | 73616 |
| 1024 to 2047 (16.0 to 32.0 µs) | 60873 |
| 2048 to 4095 (32.0 to 64.0 µs) | 55596 |
| 4096 to 8191 (64.0 to 128.0 µs) | 52449 |
| 8192 to 16383 (128.0 to 256.0 µs) | 14891 |
| 16384 to 32767 (256.0 to 512.0 µs) | 12 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 49047 | 17918 / 280.0 |
| timer | 45237 | 13169 / 205.8 |
| job_end | 9303878 | 5026 / 78.5 |
| epilogue | 22323 | 3859 / 60.3 |

## The Kernel's events

55 events mirrored (55 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 18, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 332067 | 18175 | 0.310 |
| 1 | 24982 | 6001 | 0.100 |
| 2 | 11197026 | 31972 | 12.290 |
| 3 | 1372894 | 5408 | 3.120 |
| 4 | 43672 | 10000 | 0.150 |
| 5 | 67387 | 7243 | 0.170 |
| 6 | 19995 | 1803 | 0.300 |
| none | 4939049 | 32543 | 7.260 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_timing_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_timing_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8992
log=4500
pulse=9999
echo=5999
blocked_releases=8994
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=914
loop:part0:sensor:late=2468
loop:part1:spin_ms:spin=0
loop:part1:log:busy=228
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
     Finished in 8.31s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 85 % (12 of 14 branch outcomes), part1.log 92 % (13 of 14 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (6 of 12 branch outcomes), part2.sampler 50 % (7 of 14 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 6 | 4 | 4 | 100 % | pass |
| `part0.echo` | 4 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 9 | 14 | 12 | 85 % | pass |
| `part0.sensor#cs:state` | 9 | 14 | 12 | 85 % | pass |
| `part0.sensor#cs:in_cs` | 9 | 14 | 12 | 85 % | pass |
| `part1.log` | 7 | 14 | 13 | 92 % | pass |
| `part1.pulse` | 6 | 6 | 6 | 100 % | pass |
| `part2.filter` | 14 | 12 | 6 | 50 % | pass |
| `part2.filter#cs:stats` | 14 | 12 | 6 | 50 % | pass |
| `part2.sampler` | 14 | 14 | 7 | 50 % | pass |
| `part2.sampler#cs:stats` | 14 | 14 | 7 | 50 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::traffic_step` | 10 | 5 | 117:12 false, 123:8 true, 127:12 true, 127:12 false, 147:5 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 253:12 false |
| `p1_demo_part2::filter` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 518:8 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::validate_req` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 6 | 5 | 418:72 false |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 6 | 5 | 423:77 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 2 | 2 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 385:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
