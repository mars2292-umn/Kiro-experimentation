# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 193 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5964 cycles (93.2 µs) against J_rel 12500 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 14282 cycles (223.2 µs) over 5623359 spans, closed by a timer entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10726 rises and 10000 response edges in the capture); latency 90 to 6840 µs, median 100 µs, 0 later than 15 ms; Δ_stamp observed 3984 cycles (62.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 43.1 % of the run, the Kernel 35.2 % over 1490145 entries; runs per Task [18031, 5999, 20999, 5501, 9999, 7246, 1800] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 28160 cycles (0.440 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 27520 cycles (0.430 ms at 100000 samples/s; 18031 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 472 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 240 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5964 / 93.2 | 3527 | 2^12:17845 2^13:154 |
| 2 | 8999 | 5698 / 89.0 | 3282 | 2^12:8997 2^13:2 |
| 3 | 4499 | 5378 / 84.0 | 3607 | 2^12:4397 2^13:102 |
| 5 | 7199 | 4128 / 64.5 | 3591 | 2^12:7050 2^13:149 |
| 6 | 1799 | 4110 / 64.2 | 3909 | 2^12:1699 2^13:100 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.715 | 0 | 0 | 0 |
| 1 | 5999 | 0.287 | 0 | 0 | 0 |
| 2 | 8999 | 6.893 | 0 | 0 | 0 |
| 3 | 4500 | 4.105 | 0 | 0 | 0 |
| 4 | 9999 | 6.837 | 10000 | 2195 | 2021 |
| 5 | 7200 | 7.144 | 0 | 0 | 0 |
| 6 | 1800 | 4.461 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 28160 / 0.440 (min 13450, mean 17351) | 0 above the Budget; 2^14:6524 2^15:11476 |
| Task 1 entry | 5999 | 6947 / 0.109 (min 3115, mean 3277) | 0 above the Budget; 2^12:5885 2^13:114 |
| Task 2 entry | 8999 | 407060 / 6.360 (min 399576, mean 401158) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 209746 / 3.277 (min 196466, mean 204815) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 6978 / 0.109 (min 3280, mean 3332) | 0 above the Budget; 2^12:9986 2^13:13 |
| Task 5 entry | 7200 | 11367 / 0.178 (min 6459, mean 6561) | 0 above the Budget; 2^13:7147 2^14:53 |
| Task 6 entry | 1800 | 19591 / 0.306 (min 7655, mean 10084) | 0 above the Budget; 2^13:607 2^14:1192 2^15:1 |
| Task 0 on Resource 0 | 18000 | 6972 / 0.109 | |
| Task 0 on Resource 1 | 18000 | 7026 / 0.110 | |
| Task 2 on Resource 0 | 8999 | 389309 / 6.083 | |
| Task 2 on Resource 1 | 17998 | 4072 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 5284 / 0.083 | |
| Task 6 on Resource 2 | 1800 | 4160 / 0.065 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2724 / 42.56 | 2584 |
| timer | 55873 | 3870 / 60.47 | 3031 |
| intake | 10000 | 2320 / 36.25 | 2145 |
| lock | 71997 | 461 / 7.20 | 459 |
| unlock | 71997 | 1829 / 28.58 | 1636 |
| job_end | 56497 | 2042 / 31.91 | 1772 |
| now | 5311875 | 238 / 3.72 | 236 |
| raise | 6000 | 2082 / 32.53 | 1977 |
| dma | 1808 | 824 / 12.88 | 502 |
| epilogue | 56497 | 1201 / 18.77 | 580 |
| view | 69540 | 308 / 4.81 | 275 |
| send | 27001 | 2173 / 33.95 | 2028 |
| recv | 35997 | 725 / 11.33 | 670 |

Entries that interrupted an executing Job: Task 0: 12483, Task 1: 114, Task 2: 29141, Task 3: 2870, Task 4: 13, Task 5: 98, Task 6: 1195

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3896 cycles
- Task 0 at 179940.000 ms, 3226 cycles
- Task 2 at 179945.000 ms, 3226 cycles
- Task 0 at 179950.000 ms, 3972 cycles
- Task 5 at 179950.000 ms, 3972 cycles
- Task 0 at 179960.000 ms, 3466 cycles
- Task 3 at 179960.000 ms, 3466 cycles
- Task 2 at 179965.000 ms, 3226 cycles
- Task 0 at 179970.000 ms, 3360 cycles
- Task 5 at 179975.000 ms, 3222 cycles
- Task 0 at 179980.000 ms, 3226 cycles
- Task 0 at 179990.000 ms, 3118 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5623359 of them, the longest 14282 cycles (223.2 µs), closed by a timer entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 5302766 |
| 512 to 1023 (8.0 to 16.0 µs) | 109036 |
| 1024 to 2047 (16.0 to 32.0 µs) | 59022 |
| 2048 to 4095 (32.0 to 64.0 µs) | 85510 |
| 4096 to 8191 (64.0 to 128.0 µs) | 55048 |
| 8192 to 16383 (128.0 to 256.0 µs) | 11977 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 47028 | 13216 / 206.5 |
| timer | 38111 | 14282 / 223.2 |
| job_end | 5504876 | 4823 / 75.4 |
| epilogue | 33344 | 3870 / 60.5 |

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 470045 | 18031 | 0.430 |
| 1 | 24884 | 5999 | 0.100 |
| 2 | 5688125 | 20999 | 6.230 |
| 3 | 1439088 | 5501 | 3.270 |
| 4 | 40519 | 9999 | 0.100 |
| 5 | 65513 | 7246 | 0.160 |
| 6 | 26174 | 1800 | 0.220 |
| none | 10242200 | 43446 | 8.540 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_measure/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_measure/thumbv7em-none-eabihf/release/p1-demo

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
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=472
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=240
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
     Finished in 8.46s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 83 % (5 of 6 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 93 % (15 of 16 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (6 of 12 branch outcomes), part2.sampler 50 % (7 of 14 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 26 | 6 | 5 | 83 % | pass |
| `part0.control#cs:state` | 26 | 6 | 5 | 83 % | pass |
| `part0.control#cs:in_cs` | 26 | 6 | 5 | 83 % | pass |
| `part0.echo` | 18 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 22 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 22 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 22 | 6 | 5 | 83 % | pass |
| `part1.log` | 22 | 16 | 15 | 93 % | pass |
| `part1.pulse` | 19 | 6 | 6 | 100 % | pass |
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
| `p1_demo_part1::log` | 8 | 7 | 280:72 false |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::raw` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S0>::try_recv` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S1>::send_rep` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S2>::finish` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::State::is_empty` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::State::initial` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S2>>>::from` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 310:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 6 | 5 | 181:12 false |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::raw` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S0>::send_req` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S0>::send_stop` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S1>::try_recv` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::State::is_empty` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S2>::finish` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::State::initial` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_validate::telemetry_ctl$u20$as$u20$rsk::Validate<u32>>::accept` | 0 | 0 |  |
