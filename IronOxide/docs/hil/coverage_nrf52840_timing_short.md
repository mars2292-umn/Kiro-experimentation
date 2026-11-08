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
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5989 cycles (93.6 µs) against J_rel 12500 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16425 cycles (256.6 µs) over 9936343 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10640 rises and 10001 response edges in the capture); latency 90 to 12450 µs, median 2470 µs, 0 later than 15 ms; Δ_stamp observed 4088 cycles (63.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.5 % of the run, the Kernel 56.1 % over 2398405 entries; runs per Task [18127, 6003, 32013, 5457, 10000, 7247, 1816] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20376 cycles (0.318 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18127 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 964 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 2590 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 242 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5989 / 93.6 | 3682 | 2^12:16974 2^13:1025 |
| 2 | 8991 | 5989 / 93.6 | 3725 | 2^12:8081 2^13:910 |
| 3 | 4499 | 4918 / 76.8 | 3867 | 2^12:3590 2^13:909 |
| 5 | 7199 | 5987 / 93.5 | 3756 | 2^12:6188 2^13:1011 |
| 6 | 1799 | 5082 / 79.4 | 4166 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.804 | 0 | 0 | 0 |
| 1 | 5999 | 0.296 | 0 | 0 | 0 |
| 2 | 8992 | 54.421 | 0 | 0 | 0 |
| 3 | 4500 | 6.951 | 0 | 0 | 0 |
| 4 | 10000 | 12.476 | 10000 | 2250 | 2057 |
| 5 | 7200 | 11.403 | 0 | 0 | 0 |
| 6 | 1800 | 56.803 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20376 / 0.318 (min 9710, mean 12535) | 0 above the Budget; 2^14:17586 2^15:414 |
| Task 1 entry | 5999 | 6991 / 0.109 (min 3191, mean 3283) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8992 | 2944762 / 46.012 (min 783496, mean 789169) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201606 / 3.150 (min 181793, mean 195321) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 11008 / 0.172 (min 3352, mean 3571) | 0 above the Budget; 2^12:9757 2^13:239 2^14:4 |
| Task 5 entry | 7200 | 13869 / 0.217 (min 6575, mean 6794) | 0 above the Budget; 2^13:7093 2^14:107 |
| Task 6 entry | 1800 | 23939 / 0.374 (min 7719, mean 8022) | 0 above the Budget; 2^13:1683 2^14:115 2^15:2 |
| Task 0 on Resource 0 | 18000 | 6973 / 0.109 | |
| Task 0 on Resource 1 | 18000 | 4200 / 0.066 | |
| Task 2 on Resource 0 | 8992 | 773616 / 12.088 | |
| Task 2 on Resource 1 | 17984 | 4110 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 5382 / 0.084 | |
| Task 6 on Resource 2 | 1800 | 5266 / 0.082 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2848 / 44.50 | 2688 |
| timer | 51238 | 4448 / 69.50 | 3072 |
| intake | 10000 | 2372 / 37.06 | 2181 |
| lock | 71976 | 466 / 7.28 | 464 |
| unlock | 71976 | 1880 / 29.38 | 1670 |
| job_end | 56491 | 2128 / 33.25 | 1870 |
| now | 9699428 | 233 / 3.64 | 232 |
| raise | 6000 | 2137 / 33.39 | 2024 |
| dma | 1808 | 806 / 12.59 | 507 |
| epilogue | 56491 | 1204 / 18.81 | 726 |
| enforce | 8 | 2996 / 46.81 | 2950 |
| view | 80532 | 308 / 4.81 | 273 |

Entries that interrupted an executing Job: Task 0: 8907, Task 1: 3, Task 2: 54364, Task 3: 2643, Task 4: 247, Task 5: 156, Task 6: 135

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3473 cycles
- Task 2 at 179940.000 ms, 3473 cycles
- Task 0 at 179950.000 ms, 3887 cycles
- Task 5 at 179950.000 ms, 3887 cycles
- Task 0 at 179960.000 ms, 3717 cycles
- Task 2 at 179960.000 ms, 3717 cycles
- Task 3 at 179960.000 ms, 3717 cycles
- Task 0 at 179970.000 ms, 3471 cycles
- Task 5 at 179975.000 ms, 3543 cycles
- Task 0 at 179980.000 ms, 3473 cycles
- Task 2 at 179980.000 ms, 3473 cycles
- Task 0 at 179990.000 ms, 3663 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 9936343 of them, the longest 16425 cycles (256.6 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 9678914 |
| 512 to 1023 (8.0 to 16.0 µs) | 73519 |
| 1024 to 2047 (16.0 to 32.0 µs) | 48159 |
| 2048 to 4095 (32.0 to 64.0 µs) | 67256 |
| 4096 to 8191 (64.0 to 128.0 µs) | 57200 |
| 8192 to 16383 (128.0 to 256.0 µs) | 11294 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 49339 | 16425 / 256.6 |
| timer | 45242 | 14697 / 229.6 |
| job_end | 9819480 | 4119 / 64.4 |
| epilogue | 22282 | 3945 / 61.6 |

## The Kernel's events

55 events mirrored (55 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 335043 | 18127 | 0.310 |
| 1 | 25207 | 6003 | 0.100 |
| 2 | 11190517 | 32013 | 12.300 |
| 3 | 1372767 | 5457 | 3.120 |
| 4 | 44407 | 10000 | 0.160 |
| 5 | 67849 | 7247 | 0.200 |
| 6 | 20325 | 1816 | 0.300 |
| none | 4942805 | 32447 | 7.270 |

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
blocked_releases=8994
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=964
loop:part0:sensor:late=2590
loop:part1:spin_ms:spin=0
loop:part1:log:busy=242
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
     Finished in 7.82s
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
| `p1_demo_part1::log` | 6 | 5 | 280:72 false |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 6 | 5 | 348:77 false |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 310:80 false |
| `p1_demo_part0::sensor::{closure}` | 2 | 2 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
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
