# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5634 cycles (88.0 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10640 rises and 10001 response edges in the capture); latency 90 to 12420 µs, median 2440 µs, 0 later than 15 ms; Δ_stamp observed 3960 cycles (61.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.2 % of the run, the Kernel 50.0 % over 258080 entries; runs per Task [18169, 5999, 31955, 5481, 9999, 7268, 1803] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 18676 cycles (0.292 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 18560 cycles (0.290 ms at 100000 samples/s; 18169 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1179 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3181 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 296 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5426 / 84.8 | 3466 | 2^12:17881 2^13:118 |
| 2 | 8991 | 5426 / 84.8 | 3514 | 2^12:8883 2^13:108 |
| 3 | 4499 | 5426 / 84.8 | 3653 | 2^12:4388 2^13:111 |
| 5 | 7199 | 5634 / 88.0 | 3499 | 2^12:7086 2^13:113 |
| 6 | 1799 | 4765 / 74.5 | 3945 | 2^12:1692 2^13:107 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.544 | 0 | 0 | 0 |
| 1 | 5999 | 0.283 | 0 | 0 | 0 |
| 2 | 8992 | 54.312 | 0 | 0 | 0 |
| 3 | 4500 | 6.743 | 0 | 0 | 0 |
| 4 | 10000 | 12.436 | 10000 | 2153 | 1956 |
| 5 | 7200 | 11.296 | 0 | 0 | 0 |
| 6 | 1800 | 43.556 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18676 / 0.292 (min 8978, mean 11637) | 0 above the Budget; 2^14:17996 2^15:4 |
| Task 1 entry | 5999 | 6543 / 0.102 (min 3009, mean 3109) | 0 above the Budget; 2^12:5995 2^13:4 |
| Task 2 entry | 8992 | 2947779 / 46.059 (min 782365, mean 787456) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 203810 / 3.185 (min 187060, mean 194860) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 10147 / 0.159 (min 3173, mean 3392) | 0 above the Budget; 2^12:9751 2^13:248 2^14:1 |
| Task 5 entry | 7200 | 13191 / 0.206 (min 6125, mean 6433) | 0 above the Budget; 2^13:7002 2^14:198 |
| Task 6 entry | 1800 | 18914 / 0.296 (min 7129, mean 9561) | 0 above the Budget; 2^13:505 2^14:1293 2^15:2 |
| Task 0 on Resource 0 | 18000 | 6846 / 0.107 | |
| Task 0 on Resource 1 | 18000 | 3809 / 0.060 | |
| Task 2 on Resource 0 | 8992 | 772907 / 12.077 | |
| Task 2 on Resource 1 | 17984 | 3656 / 0.057 | |
| Task 5 on Resource 2 | 7200 | 7145 / 0.112 | |
| Task 6 on Resource 2 | 1800 | 3726 / 0.058 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2757 / 43.08 | 2586 |
| timer | 52774 | 4266 / 66.66 | 2898 |
| intake | 10000 | 2271 / 35.48 | 2074 |
| lock | 71976 | 485 / 7.58 | 483 |
| unlock | 71976 | 1864 / 29.12 | 1658 |
| job_end | 56491 | 2129 / 33.27 | 1840 |
| now | 11863348 | 239 / 3.73 | 238 |
| raise | 6000 | 2128 / 33.25 | 2007 |
| dma | 1808 | 818 / 12.78 | 508 |
| epilogue | 56491 | 1208 / 18.88 | 731 |
| enforce | 8 | 2942 / 45.97 | 2807 |
| view | 80502 | 310 / 4.84 | 276 |

Entries that interrupted an executing Job: Task 0: 8939, Task 1: 4, Task 2: 54484, Task 3: 2609, Task 4: 250, Task 5: 353, Task 6: 1299

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3270 cycles
- Task 2 at 179940.000 ms, 3270 cycles
- Task 0 at 179950.000 ms, 3905 cycles
- Task 5 at 179950.000 ms, 3905 cycles
- Task 0 at 179960.000 ms, 3506 cycles
- Task 2 at 179960.000 ms, 3506 cycles
- Task 3 at 179960.000 ms, 3506 cycles
- Task 0 at 179970.000 ms, 3241 cycles
- Task 5 at 179975.000 ms, 3253 cycles
- Task 0 at 179980.000 ms, 3270 cycles
- Task 2 at 179980.000 ms, 3270 cycles
- Task 0 at 179990.000 ms, 3413 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 311312 | 18169 | 0.290 |
| 1 | 24216 | 5999 | 0.090 |
| 2 | 11162248 | 31955 | 12.270 |
| 3 | 1369985 | 5481 | 3.120 |
| 4 | 42621 | 9999 | 0.150 |
| 5 | 64668 | 7268 | 0.200 |
| 6 | 24778 | 1803 | 0.260 |
| none | 4999101 | 32477 | 7.280 |

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
pulse=10000
echo=5999
blocked_releases=8992
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1179
loop:part0:sensor:late=3181
loop:part1:spin_ms:spin=0
loop:part1:log:busy=296
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
     Finished in 7.75s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 85 % (12 of 14 branch outcomes), part1.log 92 % (13 of 14 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (6 of 12 branch outcomes), part2.sampler 50 % (7 of 14 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 5 | 4 | 4 | 100 % | pass |
| `part0.echo` | 3 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 8 | 14 | 12 | 85 % | pass |
| `part0.sensor#cs:state` | 8 | 14 | 12 | 85 % | pass |
| `part0.sensor#cs:in_cs` | 8 | 14 | 12 | 85 % | pass |
| `part1.log` | 7 | 14 | 13 | 92 % | pass |
| `part1.pulse` | 6 | 6 | 6 | 100 % | pass |
| `part2.filter` | 13 | 12 | 6 | 50 % | pass |
| `part2.filter#cs:stats` | 13 | 12 | 6 | 50 % | pass |
| `part2.sampler` | 13 | 14 | 7 | 50 % | pass |
| `part2.sampler#cs:stats` | 13 | 14 | 7 | 50 % | pass |

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
| `p1_demo_part2::sampler` | 2 | 1 | 407:8 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 6 | 5 | 226:72 false |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 6 | 5 | 275:77 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 237:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 2 | 2 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
