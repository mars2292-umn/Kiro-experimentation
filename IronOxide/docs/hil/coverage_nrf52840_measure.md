# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5528 cycles (86.4 µs) against J_rel 10000 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10644 rises and 10000 response edges in the capture); latency 80 to 6420 µs, median 90 µs, 0 later than 15 ms; Δ_stamp observed 3896 cycles (60.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 41.7 % of the run, the Kernel 30.6 % over 389657 entries; runs per Task [18039, 6002, 20999, 5193, 9999, 7288, 1822] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 19681 cycles (0.308 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 18560 cycles (0.290 ms at 100000 samples/s; 18039 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 569 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 289 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5528 / 86.4 | 3398 | 2^12:17996 2^13:3 |
| 2 | 8999 | 5229 / 81.7 | 3074 | 2^12:8997 2^13:2 |
| 3 | 4499 | 5009 / 78.3 | 3398 | 2^12:4497 2^13:2 |
| 5 | 7199 | 3889 / 60.8 | 3436 | 2^12:7199 |
| 6 | 1799 | 3889 / 60.8 | 3700 | 2^12:1799 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.556 | 0 | 0 | 0 |
| 1 | 5999 | 0.230 | 0 | 0 | 0 |
| 2 | 8999 | 6.766 | 0 | 0 | 0 |
| 3 | 4500 | 3.853 | 0 | 0 | 0 |
| 4 | 9999 | 6.434 | 10000 | 2074 | 1861 |
| 5 | 7200 | 7.006 | 0 | 0 | 0 |
| 6 | 1800 | 4.174 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19681 / 0.308 (min 8847, mean 11966) | 0 above the Budget; 2^14:17936 2^15:64 |
| Task 1 entry | 5999 | 6670 / 0.104 (min 3092, mean 3189) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 405399 / 6.334 (min 398471, mean 400008) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 200640 / 3.135 (min 189980, mean 195581) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 6690 / 0.105 (min 3222, mean 3275) | 0 above the Budget; 2^12:9990 2^13:9 |
| Task 5 entry | 7200 | 13786 / 0.215 (min 6130, mean 6278) | 0 above the Budget; 2^13:7065 2^14:135 |
| Task 6 entry | 1800 | 17734 / 0.277 (min 7190, mean 8845) | 0 above the Budget; 2^13:959 2^14:840 2^15:1 |
| Task 0 on Resource 0 | 18000 | 3853 / 0.060 | |
| Task 0 on Resource 1 | 18000 | 6273 / 0.098 | |
| Task 2 on Resource 0 | 8999 | 388797 / 6.075 | |
| Task 2 on Resource 1 | 17998 | 3650 / 0.057 | |
| Task 5 on Resource 2 | 7200 | 4679 / 0.073 | |
| Task 6 on Resource 2 | 1800 | 3706 / 0.058 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2799 / 43.73 | 2626 |
| timer | 55240 | 3668 / 57.31 | 2851 |
| intake | 10000 | 2196 / 34.31 | 1980 |
| lock | 71997 | 481 / 7.52 | 479 |
| unlock | 71997 | 1781 / 27.83 | 1597 |
| job_end | 56497 | 1940 / 30.31 | 1711 |
| now | 6402745 | 240 / 3.75 | 239 |
| raise | 6000 | 1973 / 30.83 | 1918 |
| dma | 1808 | 815 / 12.73 | 507 |
| epilogue | 56497 | 1209 / 18.89 | 576 |
| view | 69295 | 310 / 4.84 | 278 |

Entries that interrupted an executing Job: Task 0: 11093, Task 1: 2, Task 2: 29747, Task 3: 2391, Task 4: 9, Task 5: 220, Task 6: 888

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3468 cycles
- Task 0 at 179940.000 ms, 3025 cycles
- Task 2 at 179945.000 ms, 3021 cycles
- Task 0 at 179950.000 ms, 3800 cycles
- Task 5 at 179950.000 ms, 3800 cycles
- Task 0 at 179960.000 ms, 3257 cycles
- Task 3 at 179960.000 ms, 3257 cycles
- Task 2 at 179965.000 ms, 3021 cycles
- Task 0 at 179970.000 ms, 3548 cycles
- Task 5 at 179975.000 ms, 3017 cycles
- Task 0 at 179980.000 ms, 3025 cycles
- Task 0 at 179990.000 ms, 2909 cycles

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 314972 | 18039 | 0.290 |
| 1 | 22868 | 6002 | 0.090 |
| 2 | 5671230 | 20999 | 6.220 |
| 3 | 1372082 | 5193 | 3.110 |
| 4 | 38247 | 9999 | 0.100 |
| 5 | 61487 | 7288 | 0.150 |
| 6 | 22461 | 1822 | 0.200 |
| none | 10492990 | 43691 | 8.500 |

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
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=569
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=289
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
     Finished in 7.77s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 92 % (13 of 14 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (6 of 12 branch outcomes), part2.sampler 50 % (7 of 14 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 5 | 4 | 4 | 100 % | pass |
| `part0.echo` | 3 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 7 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 7 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 7 | 6 | 5 | 83 % | pass |
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
| `p1_demo_part0::sensor` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 237:80 false |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
