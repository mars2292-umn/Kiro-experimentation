# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5690 cycles (88.9 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10630 rises and 10000 response edges in the capture); latency 90 to 12430 µs, median 2450 µs, 0 later than 15 ms; Δ_stamp observed 4848 cycles (75.8 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.3 % of the run, the Kernel 50.5 % over 410144 entries; runs per Task [18182, 6003, 31777, 5090, 9999, 7288, 1826] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 19341 cycles (0.302 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18182 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1164 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3143 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 291 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5690 / 88.9 | 3687 | 2^12:16866 2^13:1133 |
| 2 | 8991 | 5690 / 88.9 | 3754 | 2^12:7981 2^13:1010 |
| 3 | 4499 | 5690 / 88.9 | 3894 | 2^12:3590 2^13:909 |
| 5 | 7199 | 5372 / 83.9 | 3773 | 2^12:6082 2^13:1117 |
| 6 | 1799 | 5372 / 83.9 | 4196 | 2^12:796 2^13:1003 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.716 | 0 | 0 | 0 |
| 1 | 5999 | 0.281 | 0 | 0 | 0 |
| 2 | 8992 | 54.344 | 0 | 0 | 0 |
| 3 | 4500 | 6.893 | 0 | 0 | 0 |
| 4 | 9999 | 12.448 | 10000 | 2245 | 2047 |
| 5 | 7200 | 11.335 | 0 | 0 | 0 |
| 6 | 1800 | 43.707 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19341 / 0.302 (min 9171, mean 11844) | 0 above the Budget; 2^14:17972 2^15:28 |
| Task 1 entry | 5999 | 6851 / 0.107 (min 3083, mean 3229) | 0 above the Budget; 2^12:5885 2^13:114 |
| Task 2 entry | 8992 | 2947299 / 46.052 (min 782552, mean 787696) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200961 / 3.140 (min 187287, mean 195757) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10690 / 0.167 (min 3240, mean 3636) | 0 above the Budget; 2^12:9463 2^13:322 2^14:214 |
| Task 5 entry | 7200 | 14741 / 0.230 (min 6252, mean 6552) | 0 above the Budget; 2^13:6999 2^14:201 |
| Task 6 entry | 1800 | 23358 / 0.365 (min 7263, mean 9751) | 0 above the Budget; 2^13:573 2^14:1225 2^15:2 |
| Task 0 on Resource 0 | 18000 | 7290 / 0.114 | |
| Task 0 on Resource 1 | 18000 | 4109 / 0.064 | |
| Task 2 on Resource 0 | 8992 | 773210 / 12.081 | |
| Task 2 on Resource 1 | 17984 | 3967 / 0.062 | |
| Task 5 on Resource 2 | 7200 | 4953 / 0.077 | |
| Task 6 on Resource 2 | 1800 | 8171 / 0.128 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2865 / 44.77 | 2707 |
| timer | 52586 | 4533 / 70.83 | 3140 |
| intake | 10000 | 2374 / 37.09 | 2176 |
| lock | 71976 | 506 / 7.91 | 503 |
| unlock | 71976 | 1875 / 29.30 | 1703 |
| job_end | 56490 | 2152 / 33.62 | 1892 |
| now | 11705820 | 246 / 3.84 | 244 |
| raise | 6000 | 2157 / 33.70 | 2035 |
| dma | 1808 | 836 / 13.06 | 524 |
| epilogue | 56490 | 1212 / 18.94 | 723 |
| enforce | 8 | 3069 / 47.95 | 3023 |
| view | 79984 | 309 / 4.83 | 274 |

Entries that interrupted an executing Job: Task 0: 8464, Task 1: 114, Task 2: 54012, Task 3: 2314, Task 4: 750, Task 5: 346, Task 6: 1259

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3506 cycles
- Task 2 at 179940.000 ms, 3506 cycles
- Task 0 at 179950.000 ms, 3923 cycles
- Task 5 at 179950.000 ms, 3923 cycles
- Task 0 at 179960.000 ms, 3742 cycles
- Task 2 at 179960.000 ms, 3742 cycles
- Task 3 at 179960.000 ms, 3742 cycles
- Task 0 at 179970.000 ms, 3599 cycles
- Task 5 at 179975.000 ms, 3663 cycles
- Task 0 at 179980.000 ms, 3506 cycles
- Task 2 at 179980.000 ms, 3506 cycles
- Task 0 at 179990.000 ms, 3367 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 316678 | 18182 | 0.300 |
| 1 | 25184 | 6003 | 0.100 |
| 2 | 11164260 | 31777 | 12.230 |
| 3 | 1374236 | 5090 | 3.120 |
| 4 | 46451 | 9999 | 0.160 |
| 5 | 66009 | 7288 | 0.210 |
| 6 | 25341 | 1826 | 0.270 |
| none | 4978537 | 32990 | 7.300 |

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
blocked_releases=8992
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1164
loop:part0:sensor:late=3143
loop:part1:spin_ms:spin=0
loop:part1:log:busy=291
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
     Finished in 8.06s
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
