# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7029 cycles (109.8 µs) against J_rel 10000 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10644 rises and 10000 response edges in the capture); latency 80 to 5860 µs, median 90 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 41.7 % of the run, the Kernel 30.4 % over 280836 entries; runs per Task [18039, 6002, 20999, 5098, 10077, 7241, 1801] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 19720 cycles (0.308 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 18560 cycles (0.290 ms at 100000 samples/s; 18039 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 579 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 292 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7029 / 109.8 | 3315 | 2^12:17816 2^13:183 |
| 2 | 8999 | 5744 / 89.8 | 3103 | 2^12:8949 2^13:50 |
| 3 | 4499 | 7029 / 109.8 | 3460 | 2^12:4432 2^13:67 |
| 5 | 7199 | 7029 / 109.8 | 3410 | 2^12:7137 2^13:62 |
| 6 | 1799 | 7029 / 109.8 | 3751 | 2^12:1770 2^13:29 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.568 | 0 | 0 | 0 |
| 1 | 5999 | 0.233 | 0 | 0 | 0 |
| 2 | 8999 | 6.769 | 0 | 0 | 0 |
| 3 | 4500 | 3.840 | 0 | 0 | 0 |
| 4 | 9999 | 5.877 | 10000 | 2059 | 1889 |
| 5 | 7200 | 6.999 | 0 | 0 | 0 |
| 6 | 1800 | 4.160 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19720 / 0.308 (min 8757, mean 11917) | 0 above the Budget; 2^14:17944 2^15:56 |
| Task 1 entry | 5999 | 6499 / 0.102 (min 2984, mean 3083) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8999 | 405019 / 6.328 (min 398159, mean 399948) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 200014 / 3.125 (min 190162, mean 195210) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12495 / 0.195 (min 3136, mean 3239) | 0 above the Budget; 2^12:9908 2^13:12 2^14:79 |
| Task 5 entry | 7200 | 10608 / 0.166 (min 6044, mean 6166) | 0 above the Budget; 2^13:7101 2^14:99 |
| Task 6 entry | 1800 | 17620 / 0.275 (min 7096, mean 8681) | 0 above the Budget; 2^13:975 2^14:824 2^15:1 |
| Task 0 on Resource 0 | 18000 | 6957 / 0.109 | |
| Task 0 on Resource 1 | 18000 | 3851 / 0.060 | |
| Task 2 on Resource 0 | 8999 | 388645 / 6.073 | |
| Task 2 on Resource 1 | 17998 | 3718 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 4661 / 0.073 | |
| Task 6 on Resource 2 | 1800 | 3744 / 0.059 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 2688 / 42.00 | 2528 |
| timer | 55143 | 3811 / 59.55 | 2878 |
| intake | 10000 | 2127 / 33.23 | 1957 |
| lock | 71997 | 491 / 7.67 | 489 |
| unlock | 71997 | 1787 / 27.92 | 1603 |
| job_end | 56497 | 2001 / 31.27 | 1737 |
| now | 6507500 | 239 / 3.73 | 237 |
| raise | 6000 | 2061 / 32.20 | 1958 |
| dma | 1808 | 821 / 12.83 | 510 |
| epilogue | 56497 | 1208 / 18.88 | 582 |
| view | 69212 | 310 / 4.84 | 278 |

Entries that interrupted an executing Job: Task 0: 11175, Task 1: 3, Task 2: 30315, Task 3: 2245, Task 4: 180, Task 5: 139, Task 6: 825

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3261 cycles
- Task 0 at 179940.000 ms, 3044 cycles
- Task 2 at 179945.000 ms, 3044 cycles
- Task 0 at 179950.000 ms, 3469 cycles
- Task 5 at 179950.000 ms, 3469 cycles
- Task 0 at 179960.000 ms, 3288 cycles
- Task 3 at 179960.000 ms, 3288 cycles
- Task 2 at 179965.000 ms, 3088 cycles
- Task 0 at 179970.000 ms, 3201 cycles
- Task 5 at 179975.000 ms, 3040 cycles
- Task 0 at 179980.000 ms, 3044 cycles
- Task 0 at 179990.000 ms, 2928 cycles

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 317215 | 18039 | 0.290 |
| 1 | 23241 | 6002 | 0.090 |
| 2 | 5669978 | 20999 | 6.220 |
| 3 | 1369689 | 5098 | 3.100 |
| 4 | 39797 | 10077 | 0.140 |
| 5 | 61235 | 7241 | 0.150 |
| 6 | 22168 | 1801 | 0.160 |
| none | 10493351 | 43775 | 8.660 |

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
loop:part0:sensor:hold=579
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=292
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
     Finished in 7.73s
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
