# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9116 cycles (142.4 µs) against J_rel 12500 cycles |
| Kernel events | pass | 57 events mirrored (57 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 19, release_overlap of Task 2: 19, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 18308 cycles (286.1 µs) over 9293010 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10652 rises and 10001 response edges in the capture); latency 100 to 12350 µs, median 2460 µs, 0 later than 15 ms; Δ_stamp observed 6336 cycles (99.0 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.7 % of the run, the Kernel 53.0 % over 3062994 entries; runs per Task [18160, 6004, 31763, 5005, 10135, 7287, 1824] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21182 cycles (0.331 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18160 runs for 18000 Jobs): the agent exceeds the trace beyond quantization, within a tenth |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 901 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 2429 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 225 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 9116 / 142.4 | 3768 | 2^12:16015 2^13:1982 2^14:2 |
| 2 | 8991 | 9116 / 142.4 | 3780 | 2^12:7816 2^13:1173 2^14:2 |
| 3 | 4499 | 9116 / 142.4 | 3922 | 2^12:3467 2^13:1030 2^14:2 |
| 5 | 7199 | 8073 / 126.1 | 3814 | 2^12:5498 2^13:1701 |
| 6 | 1799 | 8073 / 126.1 | 4227 | 2^12:862 2^13:937 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.999 | 0 | 0 | 0 |
| 1 | 5999 | 0.259 | 0 | 0 | 0 |
| 2 | 8992 | 54.511 | 0 | 0 | 0 |
| 3 | 4500 | 7.050 | 0 | 0 | 0 |
| 4 | 9999 | 12.371 | 10000 | 2285 | 2058 |
| 5 | 7200 | 11.460 | 0 | 0 | 0 |
| 6 | 1800 | 57.013 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 21182 / 0.331 (min 10242, mean 13143) | 0 above the Budget; 2^14:15776 2^15:2224 |
| Task 1 entry | 5999 | 7313 / 0.114 (min 3618, mean 3714) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8992 | 2940918 / 45.952 (min 784379, mean 789730) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200979 / 3.140 (min 181172, mean 196383) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 18368 / 0.287 (min 3738, mean 4143) | 0 above the Budget; 2^12:9434 2^13:429 2^14:132 2^15:4 |
| Task 5 entry | 7200 | 15809 / 0.247 (min 7008, mean 7433) | 0 above the Budget; 2^13:6725 2^14:475 |
| Task 6 entry | 1800 | 23055 / 0.360 (min 8218, mean 8417) | 0 above the Budget; 2^14:1797 2^15:3 |
| Task 0 on Resource 0 | 18000 | 7328 / 0.115 | |
| Task 0 on Resource 1 | 18000 | 4323 / 0.068 | |
| Task 2 on Resource 0 | 8992 | 773678 / 12.089 | |
| Task 2 on Resource 1 | 17984 | 6933 / 0.108 | |
| Task 5 on Resource 2 | 7200 | 5586 / 0.087 | |
| Task 6 on Resource 2 | 1800 | 5459 / 0.085 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 3311 / 51.73 | 3104 |
| timer | 50926 | 4484 / 70.06 | 3095 |
| intake | 10000 | 2397 / 37.45 | 2169 |
| lock | 71976 | 448 / 7.00 | 446 |
| unlock | 71976 | 1908 / 29.81 | 1645 |
| job_end | 56490 | 2156 / 33.69 | 1815 |
| now | 9056937 | 224 / 3.50 | 222 |
| raise | 6000 | 2112 / 33.00 | 1982 |
| dma | 1808 | 823 / 12.86 | 497 |
| epilogue | 56490 | 1236 / 19.31 | 744 |
| enforce | 8 | 3048 / 47.62 | 3004 |
| view | 80018 | 308 / 4.81 | 273 |

Entries that interrupted an executing Job: Task 0: 9339, Task 1: 2, Task 2: 53261, Task 3: 2120, Task 4: 728, Task 5: 589, Task 6: 86

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3468 cycles
- Task 2 at 179940.000 ms, 3468 cycles
- Task 0 at 179950.000 ms, 4245 cycles
- Task 5 at 179950.000 ms, 4245 cycles
- Task 0 at 179960.000 ms, 3708 cycles
- Task 2 at 179960.000 ms, 3708 cycles
- Task 3 at 179960.000 ms, 3708 cycles
- Task 0 at 179970.000 ms, 3385 cycles
- Task 5 at 179975.000 ms, 3957 cycles
- Task 0 at 179980.000 ms, 3468 cycles
- Task 2 at 179980.000 ms, 3468 cycles
- Task 0 at 179990.000 ms, 3381 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 9293010 of them, the longest 18308 cycles (286.1 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 9037003 |
| 512 to 1023 (8.0 to 16.0 µs) | 73414 |
| 1024 to 2047 (16.0 to 32.0 µs) | 59522 |
| 2048 to 4095 (32.0 to 64.0 µs) | 52034 |
| 4096 to 8191 (64.0 to 128.0 µs) | 56015 |
| 8192 to 16383 (128.0 to 256.0 µs) | 14993 |
| 16384 to 32767 (256.0 to 512.0 µs) | 29 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48598 | 18308 / 286.1 |
| timer | 45411 | 15050 / 235.2 |
| job_end | 9177410 | 4184 / 65.4 |
| epilogue | 21591 | 3907 / 61.0 |

## The Kernel's events

57 events mirrored (57 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 19, release_overlap of Task 2: 19, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 339904 | 18160 | 0.310 |
| 1 | 25316 | 6004 | 0.100 |
| 2 | 11202909 | 31763 | 12.250 |
| 3 | 1374909 | 5005 | 3.120 |
| 4 | 48049 | 10135 | 0.180 |
| 5 | 70655 | 7287 | 0.220 |
| 6 | 20244 | 1824 | 0.300 |
| none | 4914511 | 32956 | 7.190 |

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
blocked_releases=8995
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=901
loop:part0:sensor:late=2429
loop:part1:spin_ms:spin=0
loop:part1:log:busy=225
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
