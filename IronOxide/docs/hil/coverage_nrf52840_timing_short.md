# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:81f7cd859416005eab390f134f3b764d8a87353ecaa1ff6d4ce679886a792a32

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 193 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 243370 compares left to the hardware, the closest 2 ticks ahead of the counter captured after the write (2 below 4 ticks, 42 below 8, 43 below 16); 9571 pended by software, the instant reached |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7368 cycles (115.1 µs) against J_rel 12500 cycles |
| Kernel events | pass | 42 events mirrored (42 held, 0 of them the post-detach reboot's): overrun of Task 2: 6, deadline_miss of Task 0: 1, deadline_miss of Task 2: 14, release_overlap of Task 2: 14, mit_violation of Task 2: 6, debugger of Task none: 1; the trace froze at event 41 (deadline_miss of Task 0 at 123040.000 ms); the deliberate overload case: 1 deadline miss(es) of Task 0 (control) after the injected late Job of Task 2 (sensor), whose 12 ms Critical_Section then blocks control for nearly its 10 ms deadline; the trace and the mirror stop at the first, so the events after it are not judged by this step |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 41 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 1 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 1 actions recorded (1 held, 1 of them deferrals: vectors 0x28) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 18179 cycles (284.0 µs) over 7681769 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 6458 cycles (100.9 µs) over 7768691 holds, closed by a timer entry; against the record's L_kernel 11000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10731 rises and 10001 response edges in the capture); latency 100 to 12500 µs, median 2600 µs, 0 later than 15 ms; Δ_stamp observed 4532 cycles (70.8 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 73.1 % of the run, the Kernel 56.1 % over 3425966 entries; runs per Task [18130, 6001, 32064, 5437, 9999, 7254, 1837] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23452 cycles (0.366 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23040 cycles (0.360 ms at 100000 samples/s; 18130 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 740 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 1994 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 185 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7368 / 115.1 | 3561 | 2^12:17323 2^13:676 |
| 2 | 8991 | 7368 / 115.1 | 3516 | 2^12:8771 2^13:220 |
| 3 | 4499 | 7172 / 112.1 | 3650 | 2^12:4282 2^13:217 |
| 5 | 7199 | 5176 / 80.9 | 3631 | 2^12:6603 2^13:596 |
| 6 | 1799 | 5176 / 80.9 | 3941 | 2^12:1592 2^13:207 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 10.242 | 0 | 0 | 0 |
| 1 | 5999 | 0.613 | 0 | 0 | 0 |
| 2 | 8992 | 54.586 | 0 | 0 | 0 |
| 3 | 4500 | 7.071 | 0 | 0 | 0 |
| 4 | 9999 | 12.521 | 10000 | 2241 | 2025 |
| 5 | 7200 | 11.549 | 0 | 0 | 0 |
| 6 | 1800 | 57.402 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 23452 / 0.366 (min 10823, mean 13758) | 0 above the Budget; 2^14:15399 2^15:2601 |
| Task 1 entry | 5999 | 15672 / 0.245 (min 3691, mean 3802) | 0 above the Budget; 2^12:5997 2^13:1 2^14:1 |
| Task 2 entry | 8992 | 2939323 / 45.927 (min 785602, mean 791701) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 203034 / 3.172 (min 187305, mean 196130) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 7888 / 0.123 (min 3783, mean 4153) | 0 above the Budget; 2^12:9215 2^13:784 |
| Task 5 entry | 7200 | 13173 / 0.206 (min 7365, mean 7696) | 0 above the Budget; 2^13:6987 2^14:213 |
| Task 6 entry | 1800 | 24104 / 0.377 (min 8775, mean 9219) | 0 above the Budget; 2^14:1797 2^15:3 |
| Task 0 on Resource 0 | 18000 | 4510 / 0.070 | |
| Task 0 on Resource 1 | 18000 | 4429 / 0.069 | |
| Task 2 on Resource 0 | 8992 | 775098 / 12.111 | |
| Task 2 on Resource 1 | 17984 | 4425 / 0.069 | |
| Task 5 on Resource 2 | 7200 | 6133 / 0.096 | |
| Task 6 on Resource 2 | 1800 | 7886 / 0.123 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 3125 / 48.83 | 2924 |
| timer | 51984 | 4259 / 66.55 | 2852 |
| intake | 10000 | 2375 / 37.11 | 2161 |
| lock | 71976 | 458 / 7.16 | 449 |
| unlock | 71976 | 1805 / 28.20 | 1565 |
| job_end | 56490 | 2068 / 32.31 | 1760 |
| now | 7441967 | 223 / 3.48 | 220 |
| raise | 6000 | 1976 / 30.88 | 1889 |
| dma | 1808 | 813 / 12.70 | 499 |
| epilogue | 56490 | 1254 / 19.59 | 762 |
| enforce | 8 | 2863 / 44.73 | 2792 |
| view | 80594 | 311 / 4.86 | 277 |

Entries that interrupted an executing Job: Task 0: 8893, Task 1: 4, Task 2: 53979, Task 3: 2627, Task 4: 482, Task 5: 267, Task 6: 174

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 2 (timer), 6750 cycles; Task 1: 3 (dispatch, timer), 9775 cycles; Task 2: 34 (dispatch, timer, intake), 95535 cycles; Task 3: 3 (dispatch, timer, intake), 7910 cycles; Task 4: 1 (timer), 2875 cycles; Task 5: 2 (dispatch, intake), 5444 cycles; Task 6: 5 (dispatch, timer), 15317 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3232 cycles
- Task 2 at 179940.000 ms, 3232 cycles
- Task 0 at 179950.000 ms, 4195 cycles
- Task 5 at 179950.000 ms, 4195 cycles
- Task 0 at 179960.000 ms, 3472 cycles
- Task 2 at 179960.000 ms, 3472 cycles
- Task 3 at 179960.000 ms, 3472 cycles
- Task 0 at 179970.000 ms, 3187 cycles
- Task 5 at 179975.000 ms, 3387 cycles
- Task 0 at 179980.000 ms, 3236 cycles
- Task 2 at 179980.000 ms, 3236 cycles
- Task 0 at 179990.000 ms, 3711 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 7681769 of them, the longest 18179 cycles (284.0 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 7497350 |
| 1024 to 2047 (16.0 to 32.0 µs) | 57238 |
| 2048 to 4095 (32.0 to 64.0 µs) | 55354 |
| 4096 to 8191 (64.0 to 128.0 µs) | 47554 |
| 8192 to 16383 (128.0 to 256.0 µs) | 24271 |
| 16384 to 32767 (256.0 to 512.0 µs) | 2 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48695 | 18179 / 284.0 |
| timer | 46400 | 13314 / 208.0 |
| job_end | 7565280 | 5137 / 80.3 |
| epilogue | 21394 | 4197 / 65.6 |

The longest one's 5 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | job_end, an unwind follows, exception 37 pending | 0 | 2596 | 2596 / 40.6 | 0 / 0.0 |
| 2 | epilogue, exception 37 pending | 2787 | 4307 | 1520 / 23.8 | 191 / 3.0 |
| 3 | dispatch, started a Job, exception 24 pending | 4593 | 8192 | 3599 / 56.2 | 286 / 4.5 |
| 4 | timer, exception 36 pending | 8472 | 14207 | 5735 / 89.6 | 280 / 4.4 |
| 5 | dispatch, started a Job | 14504 | 18179 | 3675 / 57.4 | 297 / 4.6 |

Rust halves 17125 cycles (267.6 µs), between them 1054 cycles (16.5 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 7768691 of them, the longest 6458 cycles (100.9 µs), closed by a timer entry:

The longest was one entry: timer, exception 36 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3767 / 58.9 |
| timer | 6458 / 100.9 |
| job_end | 2309 / 36.1 |
| epilogue | 4435 / 69.3 |

## The Kernel's events

42 events mirrored (42 held, 0 of them the post-detach reboot's): overrun of Task 2: 6, deadline_miss of Task 0: 1, deadline_miss of Task 2: 14, release_overlap of Task 2: 14, mit_violation of Task 2: 6, debugger of Task none: 1; the trace froze at event 41 (deadline_miss of Task 0 at 123040.000 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 41 | 123040.000 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 362643 | 18130 | 0.360 |
| 1 | 27475 | 6001 | 0.160 |
| 2 | 11236791 | 32064 | 12.280 |
| 3 | 1376931 | 5437 | 3.140 |
| 4 | 49326 | 9999 | 0.110 |
| 5 | 74911 | 7254 | 0.180 |
| 6 | 22906 | 1837 | 0.340 |
| none | 4845878 | 32380 | 7.200 |

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
blocked_releases=8996
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=740
loop:part0:sensor:late=1994
loop:part1:spin_ms:spin=0
loop:part1:log:busy=185
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
overload_miss_at_ms=123040
overload_held_release_at_ms=123040
overload_miss_at_ms=164040
overload_held_release_at_ms=164040
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
late_events_seen=26
overload_misses=2
RSK-OK
Firmware exited successfully
     Finished in 9.13s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 81 % (13 of 16 branch outcomes), part1.log 92 % (13 of 14 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (7 of 14 branch outcomes), part2.sampler 50 % (8 of 16 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 6 | 4 | 4 | 100 % | pass |
| `part0.echo` | 4 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 9 | 16 | 13 | 81 % | pass |
| `part0.sensor#cs:state` | 9 | 16 | 13 | 81 % | pass |
| `part0.sensor#cs:in_cs` | 9 | 16 | 13 | 81 % | pass |
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
| `p1_demo_part0::sensor` | 8 | 6 | 438:38 false, 443:77 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 405:80 false |
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
