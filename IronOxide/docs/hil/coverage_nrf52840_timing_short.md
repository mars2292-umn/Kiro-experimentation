# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:2d25c2063eda0b51431e20c2e4402cf59320c19d4eeb17865800e4824ad6b2c7

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | FAIL | the run ended without RSK-OK after 193 s (console attached) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7395 cycles (115.5 µs) against J_rel 12500 cycles |
| Kernel events | pass | 40 events mirrored (40 held, 0 of them the post-detach reboot's): overrun of Task 2: 6, deadline_miss of Task 0: 1, deadline_miss of Task 2: 13, release_overlap of Task 2: 13, mit_violation of Task 2: 6, debugger of Task none: 1; the trace froze at event 39 (deadline_miss of Task 0 at 123040.021 ms); the deliberate overload case: 1 deadline miss(es) of Task 0 (control) after the injected late Job of Task 2 (sensor), whose 12 ms Critical_Section then blocks control for nearly its 10 ms deadline; the trace and the mirror stop at the first, so the events after it are not judged by this step |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 39 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 21238 cycles (331.8 µs) over 9038879 spans, closed by a timer entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10728 rises and 10000 response edges in the capture); latency 100 to 12410 µs, median 2540 µs, 0 later than 15 ms; Δ_stamp observed 4304 cycles (67.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.8 % of the run, the Kernel 53.1 % over 3313105 entries; runs per Task [18156, 5999, 31865, 5503, 9999, 7255, 1807] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21092 cycles (0.330 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 20480 cycles (0.320 ms at 100000 samples/s; 18156 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 876 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 2358 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 219 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7395 / 115.5 | 3745 | 2^12:16242 2^13:1757 |
| 2 | 8991 | 7395 / 115.5 | 3738 | 2^12:7994 2^13:997 |
| 3 | 4499 | 7395 / 115.5 | 3875 | 2^12:3553 2^13:946 |
| 5 | 7199 | 6748 / 105.4 | 3832 | 2^12:5562 2^13:1637 |
| 6 | 1799 | 6731 / 105.2 | 4180 | 2^12:884 2^13:915 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 10.150 | 0 | 0 | 0 |
| 1 | 5999 | 0.261 | 0 | 0 | 0 |
| 2 | 8992 | 54.486 | 0 | 0 | 0 |
| 3 | 4500 | 7.071 | 0 | 0 | 0 |
| 4 | 9999 | 12.429 | 10000 | 2297 | 2061 |
| 5 | 7200 | 11.430 | 0 | 0 | 0 |
| 6 | 1800 | 56.902 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 21092 / 0.330 (min 10257, mean 13265) | 0 above the Budget; 2^14:15578 2^15:2422 |
| Task 1 entry | 5999 | 7533 / 0.118 (min 3637, mean 3751) | 0 above the Budget; 2^12:5998 2^13:1 |
| Task 2 entry | 8992 | 2939412 / 45.928 (min 784497, mean 789903) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200871 / 3.139 (min 186522, mean 195792) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12041 / 0.188 (min 3728, mean 4127) | 0 above the Budget; 2^12:9348 2^13:649 2^14:2 |
| Task 5 entry | 7200 | 14575 / 0.228 (min 7012, mean 7388) | 0 above the Budget; 2^13:6946 2^14:254 |
| Task 6 entry | 1800 | 23257 / 0.363 (min 8221, mean 8397) | 0 above the Budget; 2^14:1798 2^15:2 |
| Task 0 on Resource 0 | 18000 | 7845 / 0.123 | |
| Task 0 on Resource 1 | 18000 | 4343 / 0.068 | |
| Task 2 on Resource 0 | 8992 | 773808 / 12.091 | |
| Task 2 on Resource 1 | 17984 | 4211 / 0.066 | |
| Task 5 on Resource 2 | 7200 | 5646 / 0.088 | |
| Task 6 on Resource 2 | 1800 | 4132 / 0.065 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 3304 / 51.62 | 3093 |
| timer | 50984 | 4480 / 70.00 | 3079 |
| intake | 10000 | 2421 / 37.83 | 2189 |
| lock | 71976 | 472 / 7.38 | 463 |
| unlock | 71976 | 1889 / 29.52 | 1649 |
| job_end | 56490 | 2141 / 33.45 | 1821 |
| now | 8803381 | 239 / 3.73 | 237 |
| raise | 6000 | 2062 / 32.22 | 1982 |
| dma | 1808 | 841 / 13.14 | 505 |
| epilogue | 56490 | 1243 / 19.42 | 751 |
| enforce | 8 | 2996 / 46.81 | 2978 |
| view | 80426 | 308 / 4.81 | 273 |

Entries that interrupted an executing Job: Task 0: 9268, Task 1: 1, Task 2: 53375, Task 3: 2619, Task 4: 557, Task 5: 333, Task 6: 26

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8308 cycles; Task 1: 1 (timer), 3033 cycles; Task 2: 35 (dispatch, timer, intake), 105210 cycles; Task 3: 1 (timer), 2901 cycles; Task 4: 2 (dispatch, timer), 7298 cycles; Task 5: 2 (timer), 5822 cycles; Task 6: 5 (dispatch, timer), 16409 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3503 cycles
- Task 2 at 179940.000 ms, 3503 cycles
- Task 0 at 179950.000 ms, 3904 cycles
- Task 5 at 179950.000 ms, 3904 cycles
- Task 0 at 179960.000 ms, 3679 cycles
- Task 2 at 179960.000 ms, 3679 cycles
- Task 3 at 179960.000 ms, 3679 cycles
- Task 0 at 179970.000 ms, 3820 cycles
- Task 5 at 179975.000 ms, 3952 cycles
- Task 0 at 179980.000 ms, 3447 cycles
- Task 2 at 179980.000 ms, 3447 cycles
- Task 0 at 179990.000 ms, 3836 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 9038879 of them, the longest 21238 cycles (331.8 µs), closed by a timer entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 8782506 |
| 512 to 1023 (8.0 to 16.0 µs) | 73552 |
| 1024 to 2047 (16.0 to 32.0 µs) | 57621 |
| 2048 to 4095 (32.0 to 64.0 µs) | 53085 |
| 4096 to 8191 (64.0 to 128.0 µs) | 57004 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15108 |
| 16384 to 32767 (256.0 to 512.0 µs) | 3 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48699 | 18174 / 284.0 |
| timer | 44723 | 21238 / 331.8 |
| job_end | 8923028 | 4238 / 66.2 |
| epilogue | 22429 | 3990 / 62.3 |

## The Kernel's events

40 events mirrored (40 held, 0 of them the post-detach reboot's): overrun of Task 2: 6, deadline_miss of Task 0: 1, deadline_miss of Task 2: 13, release_overlap of Task 2: 13, mit_violation of Task 2: 6, debugger of Task none: 1; the trace froze at event 39 (deadline_miss of Task 0 at 123040.021 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 39 | 123040.021 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 345215 | 18156 | 0.320 |
| 1 | 26073 | 5999 | 0.110 |
| 2 | 11208373 | 31865 | 12.270 |
| 3 | 1374102 | 5503 | 3.120 |
| 4 | 47806 | 9999 | 0.160 |
| 5 | 70598 | 7255 | 0.210 |
| 6 | 20278 | 1807 | 0.320 |
| none | 4903841 | 32548 | 7.200 |

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
loop:part0:sensor:hold=876
loop:part0:sensor:late=2358
loop:part1:spin_ms:spin=0
loop:part1:log:busy=219
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
event=7
  task=0
  at_ms=123040
  detail=0
  prs_lo=134217728
  prs_hi=0
event=8
  task=0
  at_ms=123040
  detail=0
  prs_lo=4096
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
expected_control=18000
expected_sensor=8992
expected_late_jobs=8
late_events_seen=28
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0xedba
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1532:17
    Frame 1: report_and_exit @ 0xf982
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/system/src/lib.rs:543:5
    Frame 2: safe_state @ 0xfee0
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/decl/src/nrf52840_timing_short.rs:26:1
    Frame 3: safe_state<rsk_system::Sys> @ 0xd0b8 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:624:5
    Frame 4: irq_entry<rsk_system::Sys> @ 0xd0b6 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:543:9
    Frame 5: rsk_irq_entry_rust @ 0xc00e
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-entry/src/lib.rs:154:16
    Frame 6: __covrec_DDAC2460469FEC6Eu @ 0x17c
    Frame 7: External interrupt #8 @ 0xc5a54784
    Frame 8: _p2_ecov @ 0xc5a54784
    Frame 9: _p2_ecov @ 0xc5a54782
     Finished in 8.44s
Error: Application exit
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 85 % (12 of 14 branch outcomes), part1.log 92 % (13 of 14 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (7 of 14 branch outcomes), part2.sampler 50 % (8 of 16 branch outcomes); 0 below

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
| `part2.filter` | 16 | 14 | 7 | 50 % | pass |
| `part2.filter#cs:stats` | 16 | 14 | 7 | 50 % | pass |
| `part2.sampler` | 16 | 16 | 8 | 50 % | pass |
| `part2.sampler#cs:stats` | 16 | 16 | 8 | 50 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::traffic_step` | 10 | 5 | 186:12 false, 192:8 true, 196:12 true, 196:12 false, 219:20 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 2 | 1 | 210:16 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 423:12 false |
| `p1_demo_part2::filter` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 728:8 false |
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
| `p1_demo_part1::log` | 6 | 5 | 424:72 false |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 6 | 5 | 428:77 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 390:80 false |
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
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
