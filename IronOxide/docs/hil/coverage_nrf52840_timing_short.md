# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7804 cycles (121.9 µs) against J_rel 10000 cycles |
| Kernel events | pass | 54 events mirrored (54 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 18876 cycles (294.9 µs) over 10134305 spans, closed by a dispatch entry; against the record's L_kernel 25000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10654 rises and 10000 response edges in the capture); latency 90 to 12610 µs, median 2350 µs, 0 later than 15 ms; Δ_stamp observed 3700 cycles (57.8 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.5 % of the run, the Kernel 55.7 % over 2205609 entries; runs per Task [18140, 6001, 31921, 5082, 9998, 7199, 1807] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 19844 cycles (0.310 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18140 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 984 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 2644 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 246 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7804 / 121.9 | 3648 | 2^12:16856 2^13:1143 |
| 2 | 8991 | 7804 / 121.9 | 3678 | 2^12:8082 2^13:909 |
| 3 | 4499 | 7804 / 121.9 | 3820 | 2^12:3589 2^13:910 |
| 5 | 7199 | 5345 / 83.5 | 3723 | 2^12:6073 2^13:1126 |
| 6 | 1799 | 5345 / 83.5 | 4119 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.892 | 0 | 0 | 0 |
| 1 | 5999 | 0.247 | 0 | 0 | 0 |
| 2 | 8992 | 54.410 | 0 | 0 | 0 |
| 3 | 4500 | 6.938 | 0 | 0 | 0 |
| 4 | 9999 | 12.626 | 10000 | 2163 | 1992 |
| 5 | 7200 | 11.389 | 0 | 0 | 0 |
| 6 | 1800 | 56.725 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19844 / 0.310 (min 9646, mean 12471) | 0 above the Budget; 2^14:17615 2^15:385 |
| Task 1 entry | 5999 | 7032 / 0.110 (min 3246, mean 3339) | 0 above the Budget; 2^12:5970 2^13:29 |
| Task 2 entry | 8992 | 2944649 / 46.010 (min 783527, mean 788876) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201180 / 3.143 (min 186924, mean 195909) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10833 / 0.169 (min 3383, mean 3637) | 0 above the Budget; 2^12:9645 2^13:348 2^14:6 |
| Task 5 entry | 7200 | 10487 / 0.164 (min 6553, mean 6781) | 0 above the Budget; 2^13:7061 2^14:139 |
| Task 6 entry | 1800 | 24127 / 0.377 (min 7700, mean 10016) | 0 above the Budget; 2^13:652 2^14:1145 2^15:3 |
| Task 0 on Resource 0 | 18000 | 7414 / 0.116 | |
| Task 0 on Resource 1 | 18000 | 4145 / 0.065 | |
| Task 2 on Resource 0 | 8992 | 773647 / 12.088 | |
| Task 2 on Resource 1 | 17984 | 6600 / 0.103 | |
| Task 5 on Resource 2 | 7200 | 4139 / 0.065 | |
| Task 6 on Resource 2 | 1800 | 4022 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2871 / 44.86 | 2726 |
| timer | 52428 | 4395 / 68.67 | 3034 |
| intake | 10000 | 2288 / 35.75 | 2116 |
| lock | 71976 | 482 / 7.53 | 480 |
| unlock | 71976 | 1839 / 28.73 | 1647 |
| job_end | 56490 | 2086 / 32.59 | 1850 |
| now | 9897953 | 238 / 3.72 | 236 |
| raise | 6000 | 2037 / 31.83 | 1971 |
| dma | 1808 | 828 / 12.94 | 517 |
| epilogue | 56490 | 1203 / 18.80 | 720 |
| enforce | 8 | 2911 / 45.48 | 2902 |
| view | 80009 | 309 / 4.83 | 274 |

Entries that interrupted an executing Job: Task 0: 9093, Task 1: 29, Task 2: 54108, Task 3: 2257, Task 4: 360, Task 5: 139, Task 6: 1157

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3482 cycles
- Task 2 at 179940.000 ms, 3482 cycles
- Task 0 at 179950.000 ms, 3577 cycles
- Task 5 at 179950.000 ms, 3577 cycles
- Task 0 at 179960.000 ms, 3678 cycles
- Task 2 at 179960.000 ms, 3678 cycles
- Task 3 at 179960.000 ms, 3678 cycles
- Task 0 at 179970.000 ms, 3529 cycles
- Task 5 at 179975.000 ms, 3645 cycles
- Task 0 at 179980.000 ms, 3434 cycles
- Task 2 at 179980.000 ms, 3434 cycles
- Task 0 at 179990.000 ms, 3573 cycles

## The Kernel's events

54 events mirrored (54 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 331464 | 18140 | 0.310 |
| 1 | 25245 | 6001 | 0.100 |
| 2 | 11184714 | 31921 | 12.280 |
| 3 | 1374741 | 5082 | 3.120 |
| 4 | 44826 | 9998 | 0.160 |
| 5 | 67023 | 7199 | 0.160 |
| 6 | 25747 | 1807 | 0.290 |
| none | 4942530 | 32965 | 7.230 |

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
blocked_releases=8993
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=984
loop:part0:sensor:late=2644
loop:part1:spin_ms:spin=0
loop:part1:log:busy=246
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
     Finished in 8.11s
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
