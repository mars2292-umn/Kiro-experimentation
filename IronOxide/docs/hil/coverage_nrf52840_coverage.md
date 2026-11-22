# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:d6e5b155b352e48cf06c2494a17354ab4467cf2a7a514e526c6309825c02579f

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 242895 compares left to the hardware, the closest 2 ticks ahead of the counter captured after the write (2 below 4 ticks, 14 below 8, 20 below 16); 7635 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 37333 cycles (583.3 µs) against J_rel 12500 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 94919 cycles (1483.1 µs) over 1024303 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 24379 cycles (380.9 µs) over 1112631 holds, closed by a timer entry; against the record's L_kernel 11000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 11263 rises and 10000 response edges in the capture); latency 530 to 9810 µs, median 2280 µs, 0 later than 15 ms; Δ_stamp observed 22936 cycles (358.4 µs) against the exported bound 8000 cycles EXCEEDED; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 57.7 % of the run, the Kernel 58.0 % over 1103254 entries; runs per Task [18609, 6207, 20998, 11922, 10440, 7544, 1901] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 108079 cycles (1.689 ms, cycle-counter read cost 27 cycles) against the trace pin's longest run 104960 cycles (1.640 ms at 100000 samples/s; 18609 runs for 18000 Jobs): the agent exceeds the trace beyond quantization, within a tenth |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 70 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 44 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 37333 / 583.3 | 18826 | 2^14:789 2^15:17210 |
| 2 | 8999 | 33931 / 530.2 | 18357 | 2^14:908 2^15:8091 |
| 3 | 4499 | 37333 / 583.3 | 18868 | 2^14:1 2^15:4498 |
| 5 | 7199 | 37333 / 583.3 | 19110 | 2^14:432 2^15:6767 |
| 6 | 1799 | 37333 / 583.3 | 19877 | 2^15:1799 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 5.729 | 0 | 0 | 0 |
| 1 | 5999 | 1.303 | 0 | 0 | 0 |
| 2 | 8999 | 9.429 | 0 | 0 | 0 |
| 3 | 4500 | 18.344 | 0 | 0 | 0 |
| 4 | 9999 | 9.902 | 10000 | 11480 | 10678 |
| 5 | 7200 | 14.668 | 0 | 0 | 0 |
| 6 | 1800 | 22.917 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 27 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 108079 / 1.689 (min 50087, mean 59506) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 36224 / 0.566 (min 15896, mean 18136) | 0 above the Budget; 2^14:1919 2^15:4080 |
| Task 2 entry | 8999 | 453829 / 7.091 (min 402957, mean 419285) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 323148 / 5.049 (min 81879, mean 213398) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 69878 / 1.092 (min 16354, mean 21973) | 0 above the Budget; 2^14:1271 2^15:8728 |
| Task 5 entry | 7200 | 80560 / 1.259 (min 33296, mean 36214) | 0 above the Budget; 2^15:7200 |
| Task 6 entry | 1800 | 166294 / 2.598 (min 39299, mean 44751) | 0 above the Budget; 2^15:1800 |
| Task 0 on Resource 0 | 18000 | 41158 / 0.643 | |
| Task 0 on Resource 1 | 18000 | 41213 / 0.644 | |
| Task 2 on Resource 0 | 8999 | 363605 / 5.681 | |
| Task 2 on Resource 1 | 17998 | 41606 / 0.650 | |
| Task 5 on Resource 2 | 7200 | 30402 / 0.475 | |
| Task 6 on Resource 2 | 1800 | 22200 / 0.347 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56497 | 13285 / 207.58 | 12228 |
| timer | 49538 | 17614 / 275.22 | 14139 |
| intake | 10000 | 11967 / 186.98 | 11150 |
| lock | 71997 | 1925 / 30.08 | 1924 |
| unlock | 71997 | 9424 / 147.25 | 8299 |
| job_end | 56497 | 10541 / 164.70 | 9424 |
| now | 788298 | 1111 / 17.36 | 1111 |
| raise | 6000 | 10748 / 167.94 | 10256 |
| dma | 1808 | 3707 / 57.92 | 2715 |
| epilogue | 56497 | 5194 / 81.16 | 3213 |
| view | 76800 | 1796 / 28.06 | 1571 |

Entries that interrupted an executing Job: Task 0: 3704, Task 1: 605, Task 2: 30816, Task 3: 12997, Task 4: 2904, Task 5: 1109, Task 6: 549

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer), 40385 cycles; Task 1: 1 (timer), 13707 cycles; Task 2: 5 (dispatch, timer, intake), 65054 cycles; Task 3: 8 (dispatch, timer, intake), 106414 cycles; Task 4: 3 (dispatch, timer), 43412 cycles; Task 5: 3 (dispatch, timer, intake), 37969 cycles; Task 6: 5 (dispatch, timer), 65837 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 18578 cycles
- Task 0 at 179940.000 ms, 16535 cycles
- Task 2 at 179945.000 ms, 16447 cycles
- Task 0 at 179950.000 ms, 18106 cycles
- Task 5 at 179950.000 ms, 18106 cycles
- Task 0 at 179960.000 ms, 17423 cycles
- Task 3 at 179960.000 ms, 17423 cycles
- Task 2 at 179965.000 ms, 17450 cycles
- Task 0 at 179970.000 ms, 19678 cycles
- Task 5 at 179975.000 ms, 16487 cycles
- Task 0 at 179980.000 ms, 21814 cycles
- Task 0 at 179990.000 ms, 16171 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 1024303 of them, the longest 94919 cycles (1483.1 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 2048 to 4095 (32.0 to 64.0 µs) | 846150 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1052 |
| 8192 to 16383 (128.0 to 256.0 µs) | 81853 |
| 16384 to 32767 (256.0 to 512.0 µs) | 43804 |
| 32768 to 65535 (512.0 to 1024.0 µs) | 50833 |
| 65536 to 131071 (1024.0 to 2048.0 µs) | 611 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 52512 | 94919 / 1483.1 |
| timer | 31047 | 79615 / 1244.0 |
| job_end | 912401 | 24830 / 388.0 |
| epilogue | 28343 | 19293 / 301.5 |

The longest one's 6 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | job_end, an unwind follows, exception 41 pending | 0 | 13286 | 13286 / 207.6 | 0 / 0.0 |
| 2 | epilogue, exception 24 pending | 14238 | 20531 | 6293 / 98.3 | 952 / 14.9 |
| 3 | timer, exception 41 pending | 21574 | 40494 | 18920 / 295.6 | 1043 / 16.3 |
| 4 | dispatch, started a Job, exception 22 pending | 41557 | 58763 | 17206 / 268.8 | 1063 / 16.6 |
| 5 | intake, exception 40 pending | 59809 | 76494 | 16685 / 260.7 | 1046 / 16.3 |
| 6 | dispatch, started a Job | 77557 | 94919 | 17362 / 271.3 | 1063 / 16.6 |

Rust halves 89752 cycles (1402.4 µs), between them 5167 cycles (80.7 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 1112631 of them, the longest 24379 cycles (380.9 µs), closed by a timer entry:

The longest was one entry: timer, exception 36 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 17955 / 280.5 |
| timer | 24379 / 380.9 |
| job_end | 11999 / 187.5 |
| epilogue | 21018 / 328.4 |

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 1604843 | 18609 | 1.640 |
| 1 | 147676 | 6207 | 0.530 |
| 2 | 6159502 | 20998 | 6.540 |
| 3 | 1660113 | 11922 | 4.240 |
| 4 | 308557 | 10440 | 0.910 |
| 5 | 381624 | 7544 | 1.130 |
| 6 | 119129 | 1901 | 2.020 |
| none | 7615059 | 36189 | 8.070 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_coverage/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_coverage/thumbv7em-none-eabihf/release/p1-demo

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
loop:part0:sensor:hold=70
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=44
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
     Finished in 18.80s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 92 % (13 of 14 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (7 of 14 branch outcomes), part2.sampler 50 % (8 of 16 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 6 | 4 | 4 | 100 % | pass |
| `part0.echo` | 4 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 8 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 8 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 8 | 6 | 5 | 83 % | pass |
| `part1.log` | 7 | 14 | 13 | 92 % | pass |
| `part1.pulse` | 6 | 6 | 6 | 100 % | pass |
| `part2.filter` | 16 | 14 | 7 | 50 % | pass |
| `part2.filter#cs:stats` | 16 | 14 | 7 | 50 % | pass |
| `part2.sampler` | 16 | 16 | 8 | 50 % | pass |
| `part2.sampler#cs:stats` | 16 | 16 | 8 | 50 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `Reset` | 22 | 15 | 107:35 true, 107:57 false, 111:26 false, 121:35 true, 121:57 false, 126:26 false, 141:8 false |
| `__rustc::rust_begin_unwind` | 2 | 0 | 232:8 true, 232:8 false |
| `rsk_kernel::arch::cm4::hard_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mem_manage` | 6 | 0 | 859:12 true, 859:12 false, 861:41 true, 861:41 false, 866:44 true, 866:44 false |
| `rsk_kernel::arch::cm4::pins_enter` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::safe_state` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::span_close` | 10 | 10 |  |
| `rsk_kernel::arch::cm4::usage_fault` | 2 | 0 | 896:24 true, 896:24 false |
| `rsk_kernel::arch::cm4::usage_fault::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::consumed_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry_record` | 6 | 3 | 418:8 true, 422:8 true, 426:8 true |
| `rsk_kernel::arch::cm4::pins_running` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::pushed_kinds` | 8 | 5 | 393:18 true, 398:22 false, 400:12 false |
| `rsk_kernel::arch::cm4::trace_expect` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mirror_events` | 8 | 6 | 366:22 true, 369:26 false |
| `rsk_kernel::arch::cm4::unsafe_module::arch_state` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::fault_push` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hold_enter` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hold_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::span_enter` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::span_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::svc_number` | 2 | 1 | 1267:12 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_entry` | 4 | 3 | 651:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_stamp` | 14 | 10 | 488:8 true, 488:37 true, 492:8 true, 497:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::faults_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::record_enter` | 4 | 3 | 1007:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats_intake` | 4 | 3 | 516:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::trace_frozen` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::actions_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::events_expect` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_cs_open` | 4 | 2 | 624:8 true, 624:37 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_cs_close` | 6 | 4 | 636:8 true, 636:37 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_response` | 6 | 4 | 461:8 true, 466:16 true |
| `rsk_kernel::arch::cm4::unsafe_module::trace_ram_push` | 2 | 1 | 707:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events_seen_seq` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_calibrate` | 4 | 0 | 536:11 true, 536:11 false, 540:12 true, 540:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats_job_start` | 2 | 1 | 693:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events_push_diag` | 4 | 0 | 603:8 true, 603:8 false, 614:8 true, 614:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats_interrupted` | 4 | 2 | 675:8 true, 683:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dwt_cyccnt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::kernel_enter` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::running` | 4 | 3 | 1336:28 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::vectpending` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fault_status` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stamp_capture` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::coverage_reset` | 10 | 7 | 184:19 false, 186:20 false, 186:34 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stack_pointers` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stamp_probe_init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::svc_clear_pending` | 2 | 0 | 669:8 true, 669:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::wait_for_interrupt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stacking_fault_discard` | 2 | 0 | 684:8 true, 684:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::EndpointStorage<_>::len` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::EndpointStorage<_>::base` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::boot_info` | 8 | 4 | 957:8 false, 967:12 false, 967:32 false, 970:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hold` | 2 | 1 | 950:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::span` | 2 | 1 | 904:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats` | 2 | 1 | 382:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events` | 2 | 1 | 1112:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::extend` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::faults` | 2 | 0 | 786:8 true, 786:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::actions` | 2 | 0 | 1077:8 true, 1077:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::windows` | 2 | 0 | 853:8 true, 853:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::frame_pc` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc_frame_check` | 2 | 0 | 675:8 true, 675:8 false |
| `rsk_kernel::arch::cm4::svc` | 24 | 17 | 752:12 false, 765:12 true, 765:20 true, 765:20 false, 771:12 false, 782:8 true, 786:27 true |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::boot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::plan::plan_frame` | 6 | 3 | 384:8 true, 387:8 true, 396:8 true |
| `rsk_kernel::arch::cm4::plan::top_active` | 4 | 4 |  |
| `rsk_kernel::arch::cm4::plan::compare_target` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::plan::frame_in_stack` | 4 | 2 | 363:5 false, 363:22 false |
| `rsk_kernel::arch::cm4::plan::ArchState::extend` | 2 | 1 | 291:12 true |
| `rsk_kernel::arch::cm4::entry::apply_core` | 16 | 13 | 966:48 true, 1013:12 true, 1021:12 true |
| `rsk_kernel::arch::cm4::entry::fault_core` | 4 | 0 | 4295:18 true, 4295:18 false, 4295:31 true, 4295:31 false |
| `rsk_kernel::arch::cm4::entry::raise_core` | 6 | 3 | 1606:23 false, 1611:17 false, 1623:8 false |
| `rsk_kernel::arch::cm4::entry::start_core` | 2 | 1 | 2649:8 false |
| `rsk_kernel::arch::cm4::entry::timer_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::vectors_ok` | 6 | 4 | 1068:20 true, 1079:24 true |
| `rsk_kernel::arch::cm4::entry::arm_compare` | 2 | 1 | 847:21 false |
| `rsk_kernel::arch::cm4::entry::channel_pop` | 2 | 0 | 1965:8 true, 1965:8 false |
| `rsk_kernel::arch::cm4::entry::intake_core` | 6 | 4 | 3616:43 false, 3616:72 true |
| `rsk_kernel::arch::cm4::entry::boot_vectors` | 2 | 0 | 1149:11 true, 1149:11 false |
| `rsk_kernel::arch::cm4::entry::channel_push` | 2 | 0 | 1938:8 true, 1938:8 false |
| `rsk_kernel::arch::cm4::entry::charge_point` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::partition_in` | 6 | 0 | 1707:11 true, 1707:11 false, 1712:12 true, 1712:12 false, 1712:37 true, 1712:37 false |
| `rsk_kernel::arch::cm4::entry::psp_in_stack` | 2 | 1 | 2673:8 true |
| `rsk_kernel::arch::cm4::entry::unbound_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::channel_index` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::end_plan_core` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_args` | 32 | 0 | 2167:8 true, 2167:8 false, 2167:28 true, 2167:28 false, 2167:60 true, 2167:60 false, 2167:82 true, 2167:82 false, 2167:103 true, 2167:103 false, 2167:124 true, 2167:124 false, 2171:8 true, 2171:8 false, 2171:44 true, 2171:44 false, 2171:73 true, 2171:73 false, 2176:13 true, 2176:13 false, 2177:20 true, 2177:20 false, 2178:20 true, 2178:20 false, 2179:20 true, 2179:20 false, 2180:20 true, 2180:20 false, 2181:20 true, 2181:20 false, 2186:8 true, 2186:8 false |
| `rsk_kernel::arch::cm4::entry::endpoint_slot` | 4 | 0 | 2292:8 true, 2292:8 false, 2292:41 true, 2292:41 false |
| `rsk_kernel::arch::cm4::entry::epilogue_core` | 24 | 13 | 4123:8 true, 4123:22 true, 4140:8 false, 4149:22 true, 4174:8 true, 4186:12 true, 4186:12 false, 4212:37 true, 4223:16 true, 4234:20 true, 4234:20 false |
| `rsk_kernel::arch::cm4::entry::rearm_compare` | 4 | 3 | 863:34 true |
| `rsk_kernel::arch::cm4::entry::clear_channels` | 2 | 0 | 1744:11 true, 1744:11 false |
| `rsk_kernel::arch::cm4::entry::dma_windows_ok` | 8 | 5 | 1389:12 true, 1392:12 false, 1392:29 true |
| `rsk_kernel::arch::cm4::entry::dma_write_core` | 24 | 13 | 1541:8 false, 1541:17 false, 1541:36 false, 1565:16 false, 1566:20 true, 1566:36 true, 1566:55 true, 1566:76 true, 1572:20 false, 1572:29 false, 1577:8 false |
| `rsk_kernel::arch::cm4::entry::irq_entry_core` | 22 | 10 | 3724:8 false, 3724:24 false, 3735:16 true, 3759:16 false, 3789:8 false, 3789:40 true, 3789:56 true, 3789:56 false, 3789:81 true, 3789:81 false, 3801:12 true, 3801:12 false |
| `rsk_kernel::arch::cm4::entry::rejection_core` | 4 | 0 | 2132:10 true, 2132:10 false, 2132:39 true, 2132:39 false |
| `rsk_kernel::arch::cm4::entry::svc_check_core` | 6 | 3 | 2729:8 true, 2729:31 true, 2729:66 false |
| `rsk_kernel::arch::cm4::entry::violation_core` | 34 | 0 | 2071:8 true, 2071:8 false, 2074:12 true, 2074:12 false, 2074:32 true, 2074:32 false, 2074:64 true, 2074:64 false, 2074:86 true, 2074:86 false, 2079:12 true, 2079:12 false, 2079:41 true, 2079:41 false, 2092:8 true, 2092:8 false, 2092:27 true, 2092:27 false, 2092:47 true, 2092:47 false, 2092:79 true, 2092:79 false, 2092:101 true, 2092:101 false, 2092:122 true, 2092:122 false, 2097:8 true, 2097:8 false, 2097:37 true, 2097:37 false, 2097:66 true, 2097:66 false, 2097:82 true, 2097:82 false |
| `rsk_kernel::arch::cm4::entry::copy_in_checked` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_reinit` | 12 | 0 | 1850:11 true, 1850:11 false, 1850:20 true, 1850:20 false, 1868:12 true, 1868:12 false, 1874:19 true, 1874:19 false, 1884:20 true, 1884:20 false, 1884:45 true, 1884:45 false |
| `rsk_kernel::arch::cm4::entry::watchdog_reload` | 4 | 0 | 149:8 true, 149:8 false, 154:8 true, 154:8 false |
| `rsk_kernel::arch::cm4::entry::copy_out_checked` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_request_core` | 6 | 3 | 1460:23 false, 1465:17 false, 1477:8 false |
| `rsk_kernel::arch::cm4::entry::svc_service_core` | 26 | 13 | 3902:23 false, 3903:12 false, 3917:23 false, 3923:15 true, 3937:15 true, 3937:37 true, 3943:23 true, 3943:23 false, 3949:12 true, 3949:12 false, 3949:34 true, 3949:34 false, 3956:15 false |
| `rsk_kernel::arch::cm4::entry::dispatch_job_core` | 12 | 7 | 3327:38 false, 3327:84 false, 3346:8 true, 3370:8 true, 3380:8 true |
| `rsk_kernel::arch::cm4::entry::dma_stop_partition` | 10 | 0 | 1351:11 true, 1351:11 false, 1357:12 true, 1357:12 false, 1358:16 true, 1358:16 false, 1361:16 true, 1361:16 false, 1364:16 true, 1364:16 false |
| `rsk_kernel::arch::cm4::entry::endpoint_recv_core` | 4 | 0 | 2579:8 true, 2579:8 false, 2599:16 true, 2599:16 false |
| `rsk_kernel::arch::cm4::entry::endpoint_send_core` | 4 | 0 | 2444:8 true, 2444:8 false, 2516:8 true, 2516:8 false |
| `rsk_kernel::arch::cm4::entry::endpoint_reinit_one` | 4 | 0 | 2011:11 true, 2011:11 false, 2028:11 true, 2028:11 false |
| `rsk_kernel::arch::cm4::entry::dispatch_vector_core` | 30 | 11 | 3491:8 false, 3491:24 false, 3491:49 true, 3517:8 false, 3517:24 true, 3522:15 true, 3522:15 false, 3526:16 true, 3526:16 false, 3526:30 true, 3526:30 false, 3526:56 true, 3526:56 false, 3531:12 true, 3531:12 false, 3545:8 true, 3545:18 true, 3561:43 false, 3561:72 true |
| `rsk_kernel::arch::cm4::entry::fault_frame_readable` | 4 | 0 | 2748:8 true, 2748:8 false, 2751:8 true, 2751:8 false |
| `rsk_kernel::arch::cm4::entry::stack_violation_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::svc_of_ended_job_core` | 2 | 1 | 2692:8 true |
| `rsk_kernel::arch::cm4::entry::Cm4Hw::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_boot` | 6 | 0 | 1407:11 true, 1407:11 false, 1411:12 true, 1411:12 false, 1414:12 true, 1414:12 false |
| `rsk_kernel::arch::cm4::entry::svc_core` | 12 | 9 | 4020:36 true, 4024:23 false, 4029:23 false |
| `rsk_kernel::arch::cm4::entry::boot_core` | 22 | 10 | 1227:8 true, 1258:8 true, 1261:8 true, 1264:8 false, 1267:8 true, 1271:8 true, 1279:8 true, 1287:8 false, 1305:11 true, 1305:11 false, 1305:21 true, 1305:21 false |
| `rsk_kernel::arch::cm4::entry::role_slot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::fault` | 2 | 0 | 836:8 true, 836:8 false |
| `rsk_kernel::arch::cm4::panic` | 2 | 0 | 929:8 true, 929:8 false |
| `rsk_kernel::arch::cm4::start` | 2 | 0 | 484:8 true, 484:8 false |
| `rsk_kernel::arch::cm4::timer` | 4 | 0 | 635:16 true, 635:16 false, 640:8 true, 640:8 false |
| `rsk_kernel::arch::cm4::trace` | 4 | 4 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_snap` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_snap` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_partition` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_seq_empty` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_seq_push_u8` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::no_job_in_progress` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_seq_drop_last` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_seq_push_usize` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_int` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::Mmio::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::timer0::now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::epilogue` | 2 | 1 | 813:8 true |
| `rsk_kernel::arch::cm4::fault_pc` | 2 | 0 | 688:8 true, 688:8 false |
| `rsk_kernel::arch::cm4::bus_fault` | 2 | 0 | 877:23 true, 877:23 false |
| `rsk_kernel::arch::cm4::irq_entry` | 26 | 23 | 508:37 false, 544:20 false, 557:37 false |
| `rsk_kernel::arch::bound::config_crc` | 20 | 20 |  |
| `rsk_kernel::arch::bound::layout_crc` | 8 | 8 |  |
| `rsk_kernel::arch::bound::basepri_for` | 2 | 2 |  |
| `rsk_kernel::arch::bound::encode_level` | 2 | 1 | 55:8 true |
| `rsk_kernel::arch::bound::basepri_for_ceiling` | 2 | 2 |  |
| `rsk_kernel::arch::bound::Crc32::new` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::byte` | 4 | 4 |  |
| `rsk_kernel::arch::bound::Crc32::finish` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_return` | 2 | 1 | 849:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_detail` | 2 | 1 | 991:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::safe_state` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_unlock` | 2 | 1 | 874:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::task_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::basepri_now` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_failed` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_used` | 2 | 1 | 775:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_instant` | 2 | 1 | 999:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_reinit` | 4 | 2 | 961:12 true, 965:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::timer_event` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::in_ro_window` | 2 | 1 | 1037:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::next_compare` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_of` | 2 | 0 | 783:12 true, 783:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::record_event` | 2 | 0 | 1057:12 true, 1057:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::release_info` | 2 | 1 | 762:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top_priority` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dma_permitted` | 2 | 1 | 1027:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_partition` | 2 | 1 | 1007:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::source_masked` | 2 | 1 | 952:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_stop_dma` | 4 | 2 | 972:12 true, 976:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_view` | 2 | 1 | 956:20 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::message_permitted` | 2 | 0 | 1047:12 true, 1047:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::overrun_continues` | 4 | 0 | 787:12 true, 787:12 false, 789:13 true, 789:13 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_stopped` | 2 | 1 | 1023:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::init` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::depth` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::start` | 2 | 1 | 728:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::intake` | 10 | 5 | 917:12 false, 918:16 false, 919:16 false, 920:16 false, 922:31 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::settle` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::ceiling` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_due` | 2 | 1 | 771:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_seq` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dispatch` | 4 | 2 | 838:12 false, 838:35 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::eligible` | 10 | 6 | 824:12 true, 829:9 false, 830:16 false, 831:16 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault_of` | 2 | 0 | 1063:12 true, 1063:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_info` | 2 | 1 | 746:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_kind` | 2 | 1 | 987:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_task` | 2 | 1 | 995:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::stack_at` | 2 | 1 | 816:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_lock` | 4 | 2 | 855:12 true, 855:44 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::validate` | 10 | 5 | 702:9 false, 703:16 false, 704:16 false, 705:16 false, 706:16 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_of` | 2 | 1 | 779:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::is_killed` | 2 | 1 | 820:12 false |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::task_entry` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::kernel` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::stack_bottom` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::intake_vector` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::dispatch_vector` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::endpoint_storage` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::task_entry_address` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::dma` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::signal` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::binding` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::endpoint` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::SystemTables>::stack_top` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::drivers::Mmio$u20$as$u20$rsk_kernel::arch::cm4::drivers::Regs>::read` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::drivers::Mmio$u20$as$u20$rsk_kernel::arch::cm4::drivers::Regs>::write` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::drivers::Clock$u20$as$u20$rsk_kernel::arch::cm4::drivers::Extend>::extend` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::svc_clear_pending` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::stacking_fault_discard` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::now` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::cycles` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::boot_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::boot_expect` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::signal_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::stack_bottom` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::timer_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::intake_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::endpoint_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dispatch_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::endpoint_storage` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::fp_flush_to_zero` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::task_entry_address` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::signal` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::binding` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::tick_hz` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::endpoint` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::irq_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::stack_top` | 0 | 0 |  |
| `rsk_entry::ipsr` | 0 | 0 |  |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate_slots` | 54 | 32 | 442:12 true, 448:25 false, 461:66 false, 462:60 true, 463:23 false, 463:47 false, 463:71 false, 464:21 true, 464:40 false, 464:64 false, 464:74 false, 465:21 false, 466:16 true, 474:33 false, 485:40 false, 485:51 false, 485:62 false, 486:33 true, 486:50 false, 486:61 false, 487:28 true, 496:20 true |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate_partitions` | 48 | 26 | 379:25 false, 388:31 true, 388:68 false, 389:20 true, 389:20 false, 389:64 true, 389:64 false, 390:72 true, 391:20 true, 391:66 true, 392:20 false, 393:34 true, 393:84 true, 394:20 true, 394:63 false, 395:18 false, 395:35 false, 395:47 false, 395:61 false, 405:37 false, 412:28 true, 422:20 false |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate` | 78 | 44 | 242:14 false, 242:24 false, 242:37 false, 242:50 false, 242:60 false, 243:16 false, 243:47 false, 254:25 false, 265:29 false, 265:51 false, 266:20 false, 267:51 false, 268:20 false, 269:20 false, 269:41 false, 270:20 false, 271:21 false, 271:53 true, 271:53 false, 272:20 false, 274:16 true, 284:12 true, 288:25 false, 300:18 false, 300:51 false, 300:71 false, 312:33 false, 326:28 true, 326:59 true, 350:22 false, 350:30 false, 360:12 true, 363:12 true, 366:12 true |
| `rsk_logic::logic::log::Log<_>::get` | 2 | 1 | 153:22 true |
| `rsk_logic::logic::log::Log<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::log::Log<_>::push` | 6 | 2 | 119:12 false, 120:26 true, 133:28 true, 133:28 false |
| `rsk_logic::logic::mpu::Layout<_>::check_apart` | 6 | 4 | 599:28 false, 606:16 true |
| `rsk_logic::logic::mpu::Layout<_>::validate_pair` | 50 | 28 | 712:14 false, 712:40 false, 712:65 false, 712:91 false, 715:14 false, 715:48 false, 715:81 false, 715:114 false, 720:12 true, 720:32 true, 725:25 false, 735:16 true, 737:25 false, 737:55 false, 742:33 false, 749:24 true, 751:31 true, 756:20 true, 764:25 false, 775:16 true, 777:25 false, 777:55 false |
| `rsk_logic::logic::mpu::Layout<_>::validate_partition` | 58 | 31 | 623:14 false, 623:39 false, 623:63 false, 627:14 false, 627:35 false, 630:12 true, 633:12 true, 636:14 false, 636:60 false, 636:104 false, 637:16 false, 637:61 false, 637:104 false, 642:14 false, 642:44 false, 648:28 false, 660:16 true, 666:32 false, 674:24 true, 679:20 true, 681:29 false, 681:60 false, 681:92 false, 682:24 false, 682:64 false, 685:27 true, 692:24 true |
| `rsk_logic::logic::mpu::Layout<_>::validate_kernel_windows` | 10 | 6 | 566:12 true, 574:28 false, 581:16 true, 583:23 true |
| `rsk_logic::logic::mpu::Layout<_>::view` | 2 | 2 |  |
| `rsk_logic::logic::mpu::Layout<_>::validate` | 30 | 17 | 503:14 false, 503:48 false, 503:83 false, 506:14 false, 506:69 false, 507:16 false, 511:12 true, 518:25 false, 528:16 true, 533:32 false, 540:26 false, 540:54 false, 545:20 true |
| `rsk_logic::logic::mpu::PartitionLayout::in_ro_window_exec` | 8 | 4 | 406:15 false, 413:16 false, 415:20 false, 415:53 false |
| `rsk_logic::logic::mpu::PartitionLayout::dma_permitted_exec` | 14 | 10 | 318:12 true, 325:12 true, 325:49 true, 350:16 true |
| `rsk_logic::logic::mpu::PartitionLayout::message_permitted_exec` | 4 | 0 | 302:12 true, 302:12 false, 309:9 true, 309:9 false |
| `rsk_logic::logic::mpu::Region::disabled` | 0 | 0 |  |
| `rsk_logic::logic::mpu::Interval::check_rules` | 6 | 3 | 139:14 false, 139:37 false, 148:9 false |
| `rsk_logic::logic::mpu::Interval::check_disjoint` | 2 | 2 |  |
| `rsk_logic::logic::mpu::pow2_u32` | 2 | 2 |  |
| `rsk_logic::logic::time::Slots<_>::arm` | 0 | 0 |  |
| `rsk_logic::logic::time::Slots<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::time::Slots<_>::next` | 8 | 8 |  |
| `rsk_logic::logic::time::Slots<_>::disarm` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::try_unmask` | 4 | 2 | 492:12 false, 492:25 true |
| `rsk_logic::logic::time::Timing<_>::job_started` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::periodic_due` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::intake_decision` | 8 | 4 | 291:12 true, 298:42 false, 300:19 true, 300:19 false |
| `rsk_logic::logic::time::Timing<_>::advance_periodic` | 4 | 3 | 263:20 true |
| `rsk_logic::logic::time::Timing<_>::deferred_due_clamped` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::new` | 2 | 2 |  |
| `rsk_logic::logic::time::Timing<_>::mask` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::defer` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::accept` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::unmask` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::finish_top` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::make_pending` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::clear_release` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::recompute_ceiling` | 4 | 3 | 878:16 false |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::new` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::kill` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::lock` | 12 | 5 | 776:12 true, 782:15 true, 787:16 true, 787:16 false, 792:12 true, 795:12 true, 803:12 false |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::defer` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::start` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::select` | 20 | 19 | 641:53 false |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::unlock` | 6 | 3 | 847:12 true, 851:12 true, 851:37 true |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::end_job` | 4 | 3 | 910:24 true |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::complete` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::reset` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::charge` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::report` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::overrun` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::remaining` | 2 | 1 | 138:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::ack_reinit` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::arm_budget` | 2 | 1 | 1937:21 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::job_return` | 10 | 6 | 2024:12 true, 2026:27 true, 2026:27 false, 2032:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::no_history` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::push_event` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::serve_slot` | 20 | 15 | 2823:19 false, 2826:23 true, 2828:23 true, 2832:23 false, 2842:20 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::svc_unlock` | 4 | 2 | 2204:12 false, 2216:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::timer_event` | 18 | 16 | 2604:12 true, 2643:60 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::ack_stop_dma` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::budget_event` | 8 | 4 | 3146:12 true, 3146:37 true, 3149:12 true, 3152:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::record_event` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::deadline_miss` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::mit_violation` | 2 | 0 | 2534:12 true, 2534:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::apply_response` | 18 | 0 | 3270:12 true, 3270:12 false, 3270:44 true, 3270:44 false, 3276:12 true, 3276:12 false, 3279:16 true, 3279:16 false, 3281:20 true, 3281:20 false, 3283:24 true, 3283:24 false, 3305:28 true, 3305:28 false, 3334:12 true, 3334:12 false, 3338:12 true, 3338:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_mit_window` | 4 | 2 | 2126:12 false, 2126:25 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::intake_release` | 16 | 6 | 2378:12 true, 2378:31 true, 2378:48 true, 2398:12 false, 2423:16 true, 2423:16 false, 2423:37 true, 2423:37 false, 2433:29 true, 2433:29 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::stop_partition` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::unmask_instant` | 2 | 1 | 2084:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_top_release` | 4 | 0 | 3390:12 true, 3390:12 false, 3393:12 true, 3393:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::deferred_release` | 6 | 0 | 3032:12 true, 3032:12 false, 3032:48 true, 3032:48 false, 3043:12 true, 3043:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::enter_safe_state` | 4 | 4 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::periodic_release` | 10 | 6 | 2905:12 true, 2916:12 false, 2917:16 false, 2918:20 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::boot_check_failed` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::restart_partition` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_partition_jobs` | 4 | 0 | 3697:12 true, 3697:12 false, 3702:16 true, 3702:16 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::kill_partition_jobs` | 6 | 0 | 3555:15 true, 3555:15 false, 3574:16 true, 3574:16 false, 3574:64 true, 3574:64 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::top_holds_nothing_exec` | 4 | 0 | 2061:15 true, 2061:15 false, 2066:16 true, 2066:16 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::clear_partition_releases` | 12 | 0 | 3612:15 true, 3612:15 false, 3623:16 true, 3623:16 false, 3632:20 true, 3632:20 false, 3632:58 true, 3632:58 false, 3648:20 true, 3648:20 false, 3648:58 true, 3648:58 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::new` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::fault` | 4 | 1 | 3203:12 false, 3203:22 true, 3203:22 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::start` | 6 | 5 | 1808:24 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::intake` | 6 | 1 | 2276:12 true, 2276:23 true, 2276:23 false, 2276:50 true, 2276:50 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::settle` | 18 | 10 | 3971:41 true, 4007:20 false, 4007:36 false, 4007:56 true, 4008:24 true, 4008:24 false, 4008:69 true, 4008:69 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_top` | 2 | 0 | 3425:12 true, 3425:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::kill_at` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::observe` | 2 | 2 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::dispatch` | 2 | 2 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::fault_of` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::svc_lock` | 4 | 2 | 2165:12 false, 2177:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::boot_info` | 4 | 2 | 3851:12 false, 3854:12 false |
| `rsk_logic::logic::kernel::fault_prs` | 10 | 5 | 142:8 true, 144:15 true, 144:38 true, 146:15 true, 146:39 true |
| `rsk_logic::logic::endpoint::Fifo::pop` | 2 | 0 | 96:12 true, 96:12 false |
| `rsk_logic::logic::endpoint::Fifo::push` | 2 | 0 | 70:12 true, 70:12 false |
| `rsk_logic::logic::endpoint::Fifo::clear` | 0 | 0 |  |
| `rsk_logic::logic::endpoint::Fifo::is_wf` | 4 | 0 | 54:9 true, 54:9 false, 54:21 true, 54:21 false |
| `rsk_busfault_rust` | 0 | 0 |  |
| `rsk_default_entry` | 0 | 0 |  |
| `rsk_epilogue_rust` | 0 | 0 |  |
| `rsk_hardfault_entry` | 0 | 0 |  |
| `rsk_irq_entry_rust` | 0 | 0 |  |
| `rsk_memmanage_rust` | 0 | 0 |  |
| `rsk_nmi_entry` | 0 | 0 |  |
| `rsk_svc_rust` | 2 | 1 | 167:12 true |
| `rsk_timer_rust` | 0 | 0 |  |
| `rsk_usagefault_rust` | 0 | 0 |  |
| `rsk_entry::interrupts` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::peek` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::peek` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::watchdog_feed` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::safe_state_idle` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::safe_state_wait` | 4 | 4 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::semihosting_exit` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::debugger_attached` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::first_memmanage` | 0 | 0 |  |
| `rsk_system::report_and_exit` | 46 | 32 | 440:19 true, 442:57 true, 495:23 false, 495:38 false, 495:53 false, 495:64 false, 495:74 false, 495:84 false, 495:100 false, 495:110 false, 496:12 false, 496:22 false, 496:35 false, 496:45 false |
| `rsk_system::print_kv` | 2 | 2 |  |
| `rsk_system::write_num` | 6 | 6 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_detail` | 2 | 1 | 991:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_instant` | 2 | 1 | 999:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_prs` | 2 | 1 | 1003:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_kind` | 2 | 1 | 987:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_task` | 2 | 1 | 995:12 false |
| `rsk_logic::logic::log::Log<_>::get` | 2 | 1 | 153:22 true |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `p1_demo_part2::traffic_step` | 10 | 5 | 186:12 false, 192:8 true, 196:12 true, 196:12 false, 219:20 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 2 | 1 | 210:16 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
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
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 405:80 false |
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
| `rsk_kernel::arch::cm4::unsafe_module::hold_leave` | 10 | 9 | 994:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::span_leave` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_exec` | 10 | 8 | 557:8 true, 575:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::action_push` | 2 | 0 | 1096:8 true, 1096:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::build_frame` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::events_push` | 22 | 10 | 1161:8 true, 1179:20 false, 1179:33 true, 1179:61 true, 1179:61 false, 1183:21 true, 1184:8 false, 1184:21 true, 1184:40 true, 1184:40 false, 1184:53 true, 1184:53 false |
| `rsk_kernel::arch::cm4::unsafe_module::frame_words` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::record_leave` | 14 | 11 | 1014:8 true, 1019:8 false, 1044:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats_release` | 10 | 6 | 431:8 true, 434:8 true, 434:23 true, 434:39 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dwt_enable` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::init` | 2 | 0 | 1316:19 true, 1316:19 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::boot_checks` | 34 | 18 | 879:8 true, 882:8 true, 885:8 true, 894:8 true, 898:8 true, 898:40 true, 901:8 true, 904:8 true, 922:12 false, 926:19 false, 933:12 true, 933:24 true, 936:12 true, 941:12 true, 945:8 false, 945:29 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_program` | 6 | 3 | 1027:8 false, 1030:8 false, 1030:24 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_enable` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::primask_set` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::rasr_encode` | 8 | 8 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::rbar_encode` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_list_off` | 2 | 0 | 1105:8 true, 1105:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_disable` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_ack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_arm` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mmio_write_u8` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::primask_clear` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::write_basepri` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fpu_set_access` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_disdefwbuf` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_rearm` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::watchdog_start` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::ppi_disable_all` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_cancel` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_set_pending` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::partition_reinit` | 12 | 0 | 247:19 true, 247:19 false, 249:20 true, 249:20 false, 249:34 true, 249:34 false, 252:20 true, 252:20 false, 256:27 true, 256:27 false, 256:39 true, 256:39 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_set_priority` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::clear_fault_status` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_clear_pending` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_nonbasethrdena` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fpu_configure_eager` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mpu_enable_background` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_prigroup_all_preempt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::priority_bits_implemented` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_kernel_exception_priorities` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::peripheral` | 4 | 2 | 81:9 false, 81:23 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::kernel_access_ok` | 2 | 0 | 113:9 true, 113:9 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::ppb_known` | 24 | 0 | 91:15 true, 91:15 false, 92:16 true, 92:16 false, 98:9 true, 98:9 false, 99:18 true, 99:18 false, 99:36 true, 99:36 false, 100:21 true, 100:21 false, 100:39 true, 100:39 false, 101:21 true, 101:21 false, 101:39 true, 101:39 false, 102:21 true, 102:21 false, 102:39 true, 102:39 false, 103:21 true, 103:21 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::barrier` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_stop` | 2 | 0 | 1088:8 true, 1088:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mpu_load` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::ack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::arm` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::rearm` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::cancel` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_write` | 2 | 1 | 1098:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats_arm` | 14 | 13 | 410:21 true |
| `rsk_kernel::arch::cm4::entry::Cm4Hw::copy_words` | 14 | 0 | 4661:12 true, 4661:12 false, 4661:28 true, 4661:28 false, 4661:44 true, 4661:44 false, 4664:12 true, 4664:12 false, 4664:36 true, 4664:36 false, 4671:15 true, 4671:15 false, 4671:26 true, 4671:26 false |
| `rsk_kernel::arch::cm4::entry::Cm4Hw::read_label` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::Cm4Hw::ram_read` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::Cm4Hw::ram_write` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::dwt_enable` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::nvic_enable` | 2 | 1 | 217:12 false |
| `rsk_kernel::arch::cm4::drivers::nvic_disable` | 2 | 1 | 230:12 false |
| `rsk_kernel::arch::cm4::drivers::set_disdefwbuf` | 2 | 0 | 308:29 true, 308:29 false |
| `rsk_kernel::arch::cm4::drivers::nvic_set_pending` | 2 | 1 | 243:12 false |
| `rsk_kernel::arch::cm4::drivers::nvic_set_priority` | 2 | 0 | 269:12 true, 269:12 false |
| `rsk_kernel::arch::cm4::drivers::nvic_clear_pending` | 2 | 1 | 256:12 false |
| `rsk_kernel::arch::cm4::drivers::set_nonbasethrdena` | 2 | 0 | 298:27 true, 298:27 false |
| `rsk_kernel::arch::cm4::drivers::fpu_configure_eager` | 2 | 0 | 339:30 true, 339:30 false |
| `rsk_kernel::arch::cm4::drivers::mpu_enable_background` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::set_prigroup_all_preempt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::set_kernel_exception_priorities` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::irq_of` | 4 | 2 | 186:8 true, 186:18 true |
| `rsk_kernel::arch::cm4::drivers::timer0::ack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::timer0::arm` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::drivers::timer0::now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::drivers::timer0::rearm` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::drivers::timer0::cancel` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::trace_tag` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::u32` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::u64` | 0 | 0 |  |
| `rsk_kernel::version::version_bytes` | 4 | 0 | 28:11 true, 28:11 false, 29:12 true, 29:12 false |
| `_<rsk_kernel::arch::cm4::drivers::Mmio$u20$as$u20$rsk_kernel::arch::cm4::drivers::Regs>::write_u8` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::boot_checks` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::apply` | 2 | 1 | 641:20 true |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::copy_in` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::copy_out` | 0 | 0 |  |
| `rsk_kernel::arch::placeholder_view` | 0 | 0 |  |
| `rsk_kernel::arch::placeholder_kernel` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::hard_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mem_manage` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::safe_state` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::span_close` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::usage_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::usage_fault::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::consumed_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry_record` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::pushed_kinds` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mirror_events` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::all_bytes_zero` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::for_static` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::peek` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::_unused` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::read_basepri` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::Words<_>::fits` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::Words<_>::zero` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::safe_state_wait` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::EndpointStorage<_>::len` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::EndpointStorage<_>::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::EndpointStorage<_>::base` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::kernel_byte_access_ok` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::ram_word` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::plausible` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc_frame_check` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::boot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::plan::ArchState::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::apply_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::fault_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::raise_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::start_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::timer_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::vectors_ok` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::arm_compare` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::intake_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::boot_vectors` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::charge_point` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::psp_in_stack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::unbound_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::end_plan_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_args` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_slot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::epilogue_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::rearm_compare` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_windows_ok` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_write_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::irq_entry_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::rejection_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::svc_check_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::violation_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::copy_in_checked` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_reinit` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::copy_out_checked` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_request_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::svc_service_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dispatch_job_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_stop_partition` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_recv_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_send_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dispatch_vector_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::fault_frame_readable` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::stack_violation_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::svc_of_ended_job_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_boot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::svc_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::boot_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::panic` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::start` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::timer` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::ghost_snap` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::ghostv::no_job_in_progress` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::epilogue` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::fault_pc` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::bus_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::irq_entry` | 0 | 0 |  |
| `rsk_kernel::arch::bound::config_crc` | 0 | 0 |  |
| `rsk_kernel::arch::bound::layout_crc` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_return` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_detail` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::safe_state` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_unlock` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::task_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::basepri_now` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_failed` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_used` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_instant` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_reinit` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::timer_event` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::in_ro_window` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::next_compare` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_of` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::record_event` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::release_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top_priority` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dma_permitted` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_partition` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::source_masked` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_stop_dma` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_view` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::message_permitted` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::overrun_continues` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_stopped` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::init` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::depth` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::start` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::intake` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::select` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::settle` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::ceiling` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_due` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_prs` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_seq` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::started` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dispatch` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::eligible` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault_of` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_kind` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_task` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::stack_at` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_lock` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::validate` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_of` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::is_killed` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::boot_expect` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::signal_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::stack_bottom` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::timer_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::intake_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::priority_bits` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::endpoint_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dispatch_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::endpoint_storage` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::fp_flush_to_zero` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::task_entry_address` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::signal` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::binding` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::tick_hz` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::endpoint` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::irq_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::stack_top` | 0 | 0 |  |

## Structural coverage of the Kernel on the Target (task 19.5, R53.3)

the Kernel on the Target: lines 61.2 % (3701 of 6046), branch outcomes 51.1 % (909 of 1780), functions 66.6 % (331 of 497) of the Kernel sources

| File | Lines | Branch outcomes | Functions |
|---|---|---|---|
| apps/p1-demo/part0/src/lib.rs | 93.3 % (42 of 45) | 90.0 % (9 of 10) | 92.3 % (12 of 13) |
| apps/p1-demo/part1/src/lib.rs | 100.0 % (47 of 47) | 92.9 % (13 of 14) | 100.0 % (8 of 8) |
| apps/p1-demo/part2/src/lib.rs | 80.7 % (67 of 83) | 50.0 % (8 of 16) | 84.2 % (16 of 19) |
| apps/p1-demo/system/src/lib.rs | 98.2 % (108 of 110) | 74.1 % (40 of 54) | 100.0 % (3 of 3) |
| crates/rsk-entry/src/lib.rs | 58.2 % (46 of 79) | 61.5 % (16 of 26) | 35.7 % (5 of 14) |
| crates/rsk-kernel/src/arch/bound.rs | 86.5 % (321 of 371) | 61.6 % (90 of 146) | 89.1 % (57 of 64) |
| crates/rsk-kernel/src/arch/cm4/drivers.rs | 53.7 % (65 of 121) | 41.7 % (10 of 24) | 56.5 % (13 of 23) |
| crates/rsk-kernel/src/arch/cm4/entry.rs | 45.7 % (610 of 1335) | 33.4 % (131 of 392) | 47.5 % (29 of 61) |
| crates/rsk-kernel/src/arch/cm4/ghostv.rs | 62.5 % (15 of 24) | - (0 of 0) | 62.5 % (5 of 8) |
| crates/rsk-kernel/src/arch/cm4/mod.rs | 62.9 % (286 of 455) | 61.6 % (69 of 112) | 70.0 % (35 of 50) |
| crates/rsk-kernel/src/arch/cm4/plan.rs | 64.7 % (55 of 85) | 66.7 % (12 of 18) | 83.3 % (5 of 6) |
| crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs | 51.8 % (212 of 409) | 40.9 % (54 of 132) | 50.7 % (38 of 75) |
| crates/rsk-kernel/src/arch/cm4/unsafe_module/mod.rs | 67.2 % (353 of 525) | 61.4 % (97 of 158) | 65.4 % (34 of 52) |
| crates/rsk-kernel/src/arch/mod.rs | 0.0 % (0 of 47) | - (0 of 0) | 0.0 % (0 of 2) |
| crates/rsk-kernel/src/version.rs | 0.0 % (0 of 12) | 0.0 % (0 of 4) | 0.0 % (0 of 1) |
| crates/rsk-logic/src/logic/budget.rs | 86.4 % (19 of 22) | 50.0 % (1 of 2) | 83.3 % (5 of 6) |
| crates/rsk-logic/src/logic/endpoint.rs | 0.0 % (0 of 41) | 0.0 % (0 of 8) | 0.0 % (0 of 4) |
| crates/rsk-logic/src/logic/kernel.rs | 53.5 % (697 of 1302) | 43.9 % (93 of 212) | 58.5 % (24 of 41) |
| crates/rsk-logic/src/logic/log.rs | 65.9 % (27 of 41) | 37.5 % (3 of 8) | 100.0 % (3 of 3) |
| crates/rsk-logic/src/logic/mod.rs | 73.5 % (125 of 170) | 56.7 % (102 of 180) | 100.0 % (3 of 3) |
| crates/rsk-logic/src/logic/mpu.rs | 82.7 % (253 of 306) | 56.8 % (109 of 192) | 92.3 % (12 of 13) |
| crates/rsk-logic/src/logic/sched.rs | 86.5 % (225 of 260) | 71.7 % (33 of 46) | 84.6 % (11 of 13) |
| crates/rsk-logic/src/logic/time.rs | 82.1 % (128 of 156) | 73.1 % (19 of 26) | 86.7 % (13 of 15) |
| **total** | 61.2 % (3701 of 6046) | 51.1 % (909 of 1780) | 66.6 % (331 of 497) |
