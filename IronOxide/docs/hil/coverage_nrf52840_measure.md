# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:52d63091eef88683dfb736e2621c639dc5103ded53a5985833b140933bb38350

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| EasyDMA footprint (HW-15) at 8 Mbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 1 Mbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 125 kbps | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| EasyDMA footprint (HW-15) at 8 Mbps on SPIM1 | pass | the transfers of 100 ms wrote the 64-byte receive buffer and neither of the 8-byte guards below and above it; the buffer was read only after its transfer was taken back and the peripheral reported STOPPED |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 196 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 288769 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (6 below 4 ticks, 91 below 8, 104 below 16); 6183 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7526 cycles (117.6 µs) against J_rel 12500 cycles |
| Kernel events | pass | 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 16064 cycles (251.0 µs) over 3889291 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 5579 cycles (87.2 µs) over 3966105 holds, closed by a timer entry; against the record's L_kernel 11000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10910 rises and 10001 response edges in the capture); latency 100 to 5530 µs, median 110 µs, 0 later than 15 ms; Δ_stamp observed 3576 cycles (55.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 39.9 % of the run, the Kernel 33.1 % over 1775018 entries; runs per Task [18038, 6000, 20995, 5373, 10136, 7202, 1802] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 30550 cycles (0.477 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 29440 cycles (0.460 ms at 100000 samples/s; 18038 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 299 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 182 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7526 / 117.6 | 3510 | 2^12:16922 2^13:1077 |
| 2 | 8999 | 5267 / 82.3 | 3090 | 2^12:8977 2^13:22 |
| 3 | 4499 | 7526 / 117.6 | 3441 | 2^12:4396 2^13:103 |
| 5 | 7199 | 7526 / 117.6 | 3503 | 2^12:6264 2^13:935 |
| 6 | 1799 | 7526 / 117.6 | 3731 | 2^12:1753 2^13:46 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.895 | 0 | 0 | 0 |
| 1 | 5999 | 0.268 | 0 | 0 | 0 |
| 2 | 8999 | 5.996 | 0 | 0 | 0 |
| 3 | 4500 | 4.537 | 0 | 0 | 0 |
| 4 | 9999 | 5.542 | 10000 | 2194 | 1976 |
| 5 | 7200 | 6.484 | 0 | 0 | 0 |
| 6 | 1800 | 4.966 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 30550 / 0.477 (min 19054, mean 21528) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 7518 / 0.117 (min 3634, mean 3755) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8999 | 345680 / 5.401 (min 337900, mean 339906) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 225252 / 3.520 (min 206640, mean 215481) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 15583 / 0.243 (min 3792, mean 4298) | 0 above the Budget; 2^12:8845 2^13:1016 2^14:138 |
| Task 5 entry | 7200 | 28792 / 0.450 (min 10996, mean 14987) | 0 above the Budget; 2^14:4892 2^15:2308 |
| Task 6 entry | 1800 | 30758 / 0.481 (min 9062, mean 12079) | 0 above the Budget; 2^14:1794 2^15:6 |
| Task 0 on Resource 0 | 18000 | 4396 / 0.069 | |
| Task 0 on Resource 1 | 18000 | 4280 / 0.067 | |
| Task 2 on Resource 0 | 8999 | 325956 / 5.093 | |
| Task 2 on Resource 1 | 17998 | 4393 / 0.069 | |
| Task 5 on Resource 2 | 7200 | 4515 / 0.071 | |
| Task 6 on Resource 2 | 1800 | 4401 / 0.069 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 3076 / 48.06 | 2869 |
| timer | 49408 | 3752 / 58.62 | 2771 |
| intake | 10000 | 2318 / 36.22 | 2111 |
| lock | 71997 | 488 / 7.62 | 486 |
| unlock | 71997 | 1808 / 28.25 | 1560 |
| job_end | 56497 | 2030 / 31.72 | 1670 |
| now | 3506632 | 239 / 3.73 | 239 |
| raise | 6000 | 2043 / 31.92 | 1933 |
| dma | 1855 | 829 / 12.95 | 562 |
| epilogue | 56497 | 1255 / 19.61 | 614 |
| view | 69505 | 311 / 4.86 | 282 |
| send | 44552 | 2242 / 35.03 | 2090 |
| recv | 90671 | 946 / 14.78 | 852 |

Entries that interrupted an executing Job: Task 0: 2769, Task 1: 3, Task 2: 28148, Task 3: 2978, Task 4: 1308, Task 5: 856, Task 6: 1505

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 2 (timer), 5080 cycles; Task 1: 1 (timer), 2762 cycles; Task 2: 4 (dispatch, timer), 11016 cycles; Task 3: 4 (dispatch, timer, intake), 10486 cycles; Task 4: 3 (dispatch, timer), 9698 cycles; Task 5: 2 (timer), 5628 cycles; Task 6: 2 (timer), 5360 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3518 cycles
- Task 0 at 179940.000 ms, 3019 cycles
- Task 2 at 179945.000 ms, 3019 cycles
- Task 0 at 179950.000 ms, 4106 cycles
- Task 5 at 179950.000 ms, 4106 cycles
- Task 0 at 179960.000 ms, 3335 cycles
- Task 3 at 179960.000 ms, 3335 cycles
- Task 2 at 179965.000 ms, 3019 cycles
- Task 0 at 179970.000 ms, 3834 cycles
- Task 5 at 179975.000 ms, 3019 cycles
- Task 0 at 179980.000 ms, 3015 cycles
- Task 0 at 179990.000 ms, 2967 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 3889291 of them, the longest 16064 cycles (251.0 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 3568879 |
| 1024 to 2047 (16.0 to 32.0 µs) | 149685 |
| 2048 to 4095 (32.0 to 64.0 µs) | 100562 |
| 4096 to 8191 (64.0 to 128.0 µs) | 62916 |
| 8192 to 16383 (128.0 to 256.0 µs) | 7249 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 53630 | 16064 / 251.0 |
| timer | 31243 | 14003 / 218.8 |
| job_end | 3771213 | 4590 / 71.7 |
| epilogue | 33205 | 7826 / 122.3 |

The longest one's 4 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | intake, exception 40 pending | 0 | 2607 | 2607 / 40.7 | 0 / 0.0 |
| 2 | dispatch, started a Job, exception 24 pending | 2894 | 6276 | 3382 / 52.8 | 287 / 4.5 |
| 3 | timer, exception 36 pending | 6552 | 12131 | 5579 / 87.2 | 276 / 4.3 |
| 4 | dispatch, started a Job | 12424 | 16064 | 3640 / 56.9 | 293 / 4.6 |

Rust halves 15208 cycles (237.6 µs), between them 856 cycles (13.4 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 3966105 of them, the longest 5579 cycles (87.2 µs), closed by a timer entry:

The longest was one entry: timer, exception 36 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3671 / 57.4 |
| timer | 5579 / 87.2 |
| job_end | 2558 / 40.0 |
| epilogue | 4384 / 68.5 |

## The Kernel's events

3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 577646 | 18038 | 0.460 |
| 1 | 26153 | 6000 | 0.110 |
| 2 | 4832515 | 20995 | 5.260 |
| 3 | 1511774 | 5373 | 3.470 |
| 4 | 51030 | 10136 | 0.160 |
| 5 | 156015 | 7202 | 0.440 |
| 6 | 30824 | 1802 | 0.470 |
| none | 10810327 | 43482 | 9.410 |

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
loop:part0:sensor:hold=299
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=182
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=1799
dma_after_stop=0
dma_footprint=1
dma_footprint_1m=1
dma_footprint_125k=1
dma_footprint_spim1=1
spim0_enable=7
spim0_shorts=131072
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
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
     Finished in 11.58s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 83 % (5 of 6 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 88 % (16 of 18 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 76 % (29 of 38 branch outcomes), part2.sampler 73 % (28 of 38 branch outcomes); 0 below

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
| `part2.filter` | 75 | 38 | 29 | 76 % | pass |
| `part2.filter#cs:stats` | 75 | 38 | 29 | 76 % | pass |
| `part2.sampler` | 72 | 38 | 28 | 73 % | pass |
| `part2.sampler#cs:stats` | 72 | 38 | 28 | 73 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::traffic_step` | 30 | 23 | 170:16 false, 178:16 false, 186:12 false, 192:8 true, 196:12 true, 196:12 false, 219:20 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 2 | 1 | 210:16 false |
| `p1_demo_part2::validate_req` | 0 | 0 |  |
| `p1_demo_part2::footprint_step` | 0 | 0 |  |
| `p1_demo_part2::footprint_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::footprint_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::footprint_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::footprint_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step` | 2 | 2 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::second_instance_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 423:12 false |
| `p1_demo_part2::filter` | 2 | 2 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 728:8 false |
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
| `p1_demo_part1::log` | 10 | 8 | 302:23 true, 424:72 false |
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
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 405:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 6 | 5 | 187:12 false |
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
