# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 196 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5719 cycles (89.4 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 13625 cycles (212.9 µs) over 4607815 spans, closed by a timer entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10909 rises and 10001 response edges in the capture); latency 100 to 5700 µs, median 110 µs, 0 later than 15 ms; Δ_stamp observed 3240 cycles (50.6 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.4 % of the run, the Kernel 30.9 % over 1764830 entries; runs per Task [18033, 6002, 20997, 5502, 9999, 7213, 1800] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 28501 cycles (0.445 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 27520 cycles (0.430 ms at 100000 samples/s; 18033 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 360 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 220 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5111 / 79.9 | 3508 | 2^12:17995 2^13:4 |
| 2 | 8999 | 5719 / 89.4 | 3276 | 2^12:8996 2^13:3 |
| 3 | 4499 | 4427 / 69.2 | 3598 | 2^12:4498 2^13:1 |
| 5 | 7199 | 4071 / 63.6 | 3603 | 2^12:7199 |
| 6 | 1799 | 4071 / 63.6 | 3914 | 2^12:1799 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.850 | 0 | 0 | 0 |
| 1 | 5999 | 0.258 | 0 | 0 | 0 |
| 2 | 8999 | 5.936 | 0 | 0 | 0 |
| 3 | 4500 | 4.442 | 0 | 0 | 0 |
| 4 | 9999 | 5.727 | 10000 | 2253 | 2010 |
| 5 | 7200 | 6.365 | 0 | 0 | 0 |
| 6 | 1800 | 4.817 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 28501 / 0.445 (min 17611, mean 20279) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7288 / 0.114 (min 3470, mean 3595) | 0 above the Budget; 2^12:5995 2^13:4 |
| Task 2 entry | 8999 | 344604 / 5.384 (min 336396, mean 338002) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 221447 / 3.460 (min 204987, mean 213484) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 7488 / 0.117 (min 3628, mean 4049) | 0 above the Budget; 2^12:8997 2^13:1002 |
| Task 5 entry | 7200 | 24516 / 0.383 (min 10092, mean 13787) | 0 above the Budget; 2^14:5172 2^15:2028 |
| Task 6 entry | 1800 | 20796 / 0.325 (min 8172, mean 10767) | 0 above the Budget; 2^13:301 2^14:1498 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7808 / 0.122 | |
| Task 0 on Resource 1 | 18000 | 4348 / 0.068 | |
| Task 2 on Resource 0 | 8999 | 325485 / 5.086 | |
| Task 2 on Resource 1 | 17998 | 4241 / 0.066 | |
| Task 5 on Resource 2 | 7200 | 5466 / 0.085 | |
| Task 6 on Resource 2 | 1800 | 4302 / 0.067 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3173 / 49.58 | 2946 |
| timer | 49127 | 3825 / 59.77 | 3008 |
| intake | 10000 | 2366 / 36.97 | 2122 |
| lock | 71997 | 461 / 7.20 | 459 |
| unlock | 71997 | 1852 / 28.94 | 1624 |
| job_end | 56497 | 2085 / 32.58 | 1726 |
| now | 4222324 | 236 / 3.69 | 233 |
| raise | 6000 | 2110 / 32.97 | 1995 |
| dma | 1808 | 842 / 13.16 | 512 |
| epilogue | 56497 | 1233 / 19.27 | 602 |
| view | 69510 | 308 / 4.81 | 275 |
| send | 44552 | 2317 / 36.20 | 2164 |
| recv | 90671 | 942 / 14.72 | 846 |

Entries that interrupted an executing Job: Task 0: 3763, Task 1: 4, Task 2: 27910, Task 3: 3060, Task 4: 1002, Task 5: 536, Task 6: 1273

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 4045 cycles
- Task 0 at 179940.000 ms, 3219 cycles
- Task 2 at 179945.000 ms, 3219 cycles
- Task 0 at 179950.000 ms, 3705 cycles
- Task 5 at 179950.000 ms, 3705 cycles
- Task 0 at 179960.000 ms, 3451 cycles
- Task 3 at 179960.000 ms, 3451 cycles
- Task 2 at 179965.000 ms, 3219 cycles
- Task 0 at 179970.000 ms, 3417 cycles
- Task 5 at 179975.000 ms, 3227 cycles
- Task 0 at 179980.000 ms, 3219 cycles
- Task 0 at 179990.000 ms, 3191 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 4607815 of them, the longest 13625 cycles (212.9 µs), closed by a timer entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4214805 |
| 512 to 1023 (8.0 to 16.0 µs) | 119316 |
| 1024 to 2047 (16.0 to 32.0 µs) | 104501 |
| 2048 to 4095 (32.0 to 64.0 µs) | 102397 |
| 4096 to 8191 (64.0 to 128.0 µs) | 61278 |
| 8192 to 16383 (128.0 to 256.0 µs) | 5518 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 54020 | 11911 / 186.1 |
| timer | 31082 | 13625 / 212.9 |
| job_end | 4489264 | 6056 / 94.6 |
| epilogue | 33449 | 3896 / 60.9 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 541865 | 18033 | 0.430 |
| 1 | 24576 | 6002 | 0.100 |
| 2 | 4801691 | 20997 | 5.250 |
| 3 | 1497990 | 5502 | 3.440 |
| 4 | 46176 | 9999 | 0.110 |
| 5 | 142490 | 7213 | 0.360 |
| 6 | 27066 | 1800 | 0.230 |
| none | 10914427 | 43476 | 9.410 |

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
relay_reqs=2250
relay_rounds=2250
report_reqs=1125
report_rounds=1125
triangle_fwd=3600
triangle_acks=3600
triangle_rounds=3600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=360
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=220
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
     Finished in 11.21s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 83 % (5 of 6 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 88 % (16 of 18 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 57 % (8 of 14 branch outcomes), part2.sampler 50 % (7 of 14 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 55 | 6 | 5 | 83 % | pass |
| `part0.control#cs:state` | 55 | 6 | 5 | 83 % | pass |
| `part0.control#cs:in_cs` | 55 | 6 | 5 | 83 % | pass |
| `part0.echo` | 45 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 49 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 49 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 49 | 6 | 5 | 83 % | pass |
| `part1.log` | 55 | 18 | 16 | 88 % | pass |
| `part1.pulse` | 50 | 6 | 6 | 100 % | pass |
| `part2.filter` | 57 | 14 | 8 | 57 % | pass |
| `part2.filter#cs:stats` | 57 | 14 | 8 | 57 % | pass |
| `part2.sampler` | 54 | 14 | 7 | 50 % | pass |
| `part2.sampler#cs:stats` | 54 | 14 | 7 | 50 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::traffic_step` | 10 | 5 | 117:12 false, 123:8 true, 127:12 true, 127:12 false, 147:5 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::validate_req` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 253:12 false |
| `p1_demo_part2::filter` | 2 | 2 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 518:8 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::raw` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<p1_demo_part2::rsk_roles::relay_sink::S0>::try_recv` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<p1_demo_part2::rsk_roles::relay_sink::S1>::send_rep` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::Sink<p1_demo_part2::rsk_roles::relay_sink::S2>::finish` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::State::is_empty` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::raw` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<p1_demo_part2::rsk_roles::report_ctl::S0>::send_req` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<p1_demo_part2::rsk_roles::report_ctl::S0>::send_stop` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<p1_demo_part2::rsk_roles::report_ctl::S1>::try_recv` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::State::is_empty` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::raw` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<p1_demo_part2::rsk_roles::triangle_sink::S0>::try_recv` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<p1_demo_part2::rsk_roles::triangle_sink::S1>::send_ack` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::Sink<p1_demo_part2::rsk_roles::triangle_sink::S2>::finish` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::State::is_empty` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::report_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::report_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::report_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::relay_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::relay_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::relay_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::triangle_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::triangle_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::triangle_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::relay_sink::State::initial` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::Ctl<p1_demo_part2::rsk_roles::report_ctl::S2>::finish` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::report_ctl::State::initial` | 0 | 0 |  |
| `p1_demo_part2::rsk_roles::triangle_sink::State::initial` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::report_ctl::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::report_ctl::Ctl<p1_demo_part2::rsk_roles::report_ctl::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::report_ctl::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::report_ctl::Ctl<p1_demo_part2::rsk_roles::report_ctl::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::report_ctl::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::report_ctl::Ctl<p1_demo_part2::rsk_roles::report_ctl::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::relay_sink::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::relay_sink::Sink<p1_demo_part2::rsk_roles::relay_sink::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::relay_sink::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::relay_sink::Sink<p1_demo_part2::rsk_roles::relay_sink::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::relay_sink::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::relay_sink::Sink<p1_demo_part2::rsk_roles::relay_sink::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::triangle_sink::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::triangle_sink::Sink<p1_demo_part2::rsk_roles::triangle_sink::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::triangle_sink::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::triangle_sink::Sink<p1_demo_part2::rsk_roles::triangle_sink::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part2::rsk_roles::triangle_sink::State$u20$as$u20$core::convert::From<p1_demo_part2::rsk_roles::triangle_sink::Sink<p1_demo_part2::rsk_roles::triangle_sink::S2>>>::from` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 10 | 8 | 296:23 true, 418:72 false |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::raw` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S0>::try_recv` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S1>::send_fwd` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S2>::send_done` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S3>::finish` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::State::is_empty` | 0 | 0 |  |
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
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::raw` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<p1_demo_part1::rsk_roles::relay_ctl::S0>::send_req` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<p1_demo_part1::rsk_roles::relay_ctl::S0>::send_stop` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<p1_demo_part1::rsk_roles::relay_ctl::S1>::try_recv` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::State::is_empty` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::relay_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::Mid<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::Mid<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::Mid<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::Mid<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::triangle_mid::State::initial` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::State::initial` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::Ctl<p1_demo_part1::rsk_roles::relay_ctl::S2>::finish` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::relay_ctl::State::initial` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::relay_ctl::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::relay_ctl::Ctl<p1_demo_part1::rsk_roles::relay_ctl::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::relay_ctl::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::relay_ctl::Ctl<p1_demo_part1::rsk_roles::relay_ctl::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::relay_ctl::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::relay_ctl::Ctl<p1_demo_part1::rsk_roles::relay_ctl::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::triangle_mid::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::triangle_mid::Mid<p1_demo_part1::rsk_roles::triangle_mid::S3>>>::from` | 0 | 0 |  |
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
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 385:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 6 | 5 | 182:12 false |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
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
| `p1_demo_part0::rsk_jobs::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::raw` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<p1_demo_part0::rsk_roles::report_sink::S0>::try_recv` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<p1_demo_part0::rsk_roles::report_sink::S1>::send_rep` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::Sink<p1_demo_part0::rsk_roles::report_sink::S2>::finish` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::State::is_empty` | 0 | 0 |  |
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
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::raw` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<p1_demo_part0::rsk_roles::triangle_ctl::S0>::send_req` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<p1_demo_part0::rsk_roles::triangle_ctl::S0>::send_stop` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<p1_demo_part0::rsk_roles::triangle_ctl::S1>::try_recv` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::State::is_empty` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::report_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::report_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::report_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::triangle_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::report_sink::State::initial` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S2>::finish` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::State::initial` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::Ctl<p1_demo_part0::rsk_roles::triangle_ctl::S2>::finish` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::triangle_ctl::State::initial` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::report_sink::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::report_sink::Sink<p1_demo_part0::rsk_roles::report_sink::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::report_sink::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::report_sink::Sink<p1_demo_part0::rsk_roles::report_sink::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::report_sink::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::report_sink::Sink<p1_demo_part0::rsk_roles::report_sink::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::triangle_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::triangle_ctl::Ctl<p1_demo_part0::rsk_roles::triangle_ctl::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::triangle_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::triangle_ctl::Ctl<p1_demo_part0::rsk_roles::triangle_ctl::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::triangle_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::triangle_ctl::Ctl<p1_demo_part0::rsk_roles::triangle_ctl::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_validate::telemetry_ctl$u20$as$u20$rsk::Validate<u32>>::accept` | 0 | 0 |  |
