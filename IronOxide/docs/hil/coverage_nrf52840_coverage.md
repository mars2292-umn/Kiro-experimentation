# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 199 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 31621 cycles (494.1 µs) against J_rel 10000 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 actions recorded (0 held): 0 deferral(s) of a dispatch vector taken inside another stub's return window |
| loopback over the run | pass | 9998 stimulus pulses answered (target counted 9998; 11062 rises and 9999 response edges in the capture); latency 400 to 9450 µs, median 1170 µs, 0 later than 15 ms; Δ_stamp observed 21824 cycles (341.0 µs) against the exported bound 8000 cycles EXCEEDED; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 55.4 % of the run, the Kernel 56.3 % over 1672105 entries; runs per Task [18521, 6037, 20995, 7325, 10043, 7218, 1887] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 107332 cycles (1.677 ms, cycle-counter read cost 27 cycles) against the trace pin's longest run 105600 cycles (1.650 ms at 100000 samples/s; 18521 runs for 18000 Jobs): the agent exceeds the trace beyond quantization, within a tenth |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 158 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 83 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 28491 / 445.2 | 15787 | 2^14:12872 2^15:5127 |
| 2 | 8999 | 31621 / 494.1 | 16681 | 2^14:6310 2^15:2689 |
| 3 | 4499 | 24019 / 375.3 | 15983 | 2^14:3596 2^15:903 |
| 5 | 7199 | 28491 / 445.2 | 16610 | 2^14:3524 2^15:3675 |
| 6 | 1799 | 28491 / 445.2 | 17109 | 2^15:1799 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 4.157 | 0 | 0 | 0 |
| 1 | 5999 | 1.291 | 0 | 0 | 0 |
| 2 | 8999 | 9.598 | 0 | 0 | 0 |
| 3 | 4500 | 16.522 | 0 | 0 | 0 |
| 4 | 9998 | 9.547 | 9999 | 9227 | 8512 |
| 5 | 7200 | 10.808 | 0 | 0 | 0 |
| 6 | 1800 | 17.674 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 27 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 107332 / 1.677 (min 36394, mean 44703) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 29375 / 0.459 (min 11549, mean 11935) | 0 above the Budget; 2^14:5987 2^15:12 |
| Task 2 entry | 8999 | 491837 / 7.685 (min 442941, mean 453197) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 271137 / 4.237 (min 134369, mean 209245) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9998 | 69530 / 1.086 (min 12011, mean 13961) | 0 above the Budget; 2^14:8958 2^15:1040 |
| Task 5 entry | 7200 | 79613 / 1.244 (min 24311, mean 27694) | 0 above the Budget; 2^15:7200 |
| Task 6 entry | 1800 | 82151 / 1.284 (min 28215, mean 35946) | 0 above the Budget; 2^15:1800 |
| Task 0 on Resource 0 | 18000 | 35192 / 0.550 | |
| Task 0 on Resource 1 | 18000 | 33699 / 0.527 | |
| Task 2 on Resource 0 | 8999 | 406128 / 6.346 | |
| Task 2 on Resource 1 | 17998 | 18472 / 0.289 | |
| Task 5 on Resource 2 | 7200 | 34252 / 0.535 | |
| Task 6 on Resource 2 | 1800 | 21757 / 0.340 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56496 | 9960 / 155.62 | 9281 |
| timer | 52350 | 15779 / 246.55 | 12742 |
| intake | 9999 | 9664 / 151.00 | 8920 |
| lock | 71997 | 1759 / 27.48 | 1757 |
| unlock | 71997 | 8052 / 125.81 | 7238 |
| job_end | 56496 | 8943 / 139.73 | 7927 |
| now | 1741876 | 920 / 14.38 | 920 |
| raise | 6000 | 9604 / 150.06 | 9012 |
| dma | 1808 | 3401 / 53.14 | 2399 |
| epilogue | 56496 | 4771 / 74.55 | 2552 |
| view | 71473 | 1776 / 27.75 | 1535 |

Entries that interrupted an executing Job: Task 0: 4677, Task 1: 12, Task 2: 32345, Task 3: 6192, Task 4: 1096, Task 5: 1342, Task 6: 888

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 16989 cycles
- Task 0 at 179940.000 ms, 14607 cycles
- Task 2 at 179945.000 ms, 14567 cycles
- Task 0 at 179950.000 ms, 16617 cycles
- Task 5 at 179950.000 ms, 16617 cycles
- Task 0 at 179960.000 ms, 15423 cycles
- Task 3 at 179960.000 ms, 15423 cycles
- Task 2 at 179965.000 ms, 22723 cycles
- Task 0 at 179970.000 ms, 16877 cycles
- Task 5 at 179975.000 ms, 14575 cycles
- Task 0 at 179980.000 ms, 14607 cycles
- Task 0 at 179990.000 ms, 14319 cycles

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 1212463 | 18521 | 1.650 |
| 1 | 98833 | 6037 | 0.430 |
| 2 | 6573527 | 20995 | 7.230 |
| 3 | 1508734 | 7325 | 3.710 |
| 4 | 188551 | 10043 | 0.750 |
| 5 | 288137 | 7218 | 1.170 |
| 6 | 96303 | 1887 | 0.950 |
| none | 8027991 | 41509 | 7.860 |

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
pulse=9998
echo=5999
blocked_releases=8999
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=158
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=83
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
     Finished in 15.22s
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
| `Reset` | 22 | 15 | 107:35 true, 107:57 false, 111:26 false, 121:35 true, 121:57 false, 126:26 false, 141:8 false |
| `__rustc::rust_begin_unwind` | 2 | 0 | 224:8 true, 224:8 false |
| `rsk_kernel::arch::cm4::hard_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mem_manage` | 6 | 0 | 712:12 true, 712:12 false, 714:41 true, 714:41 false, 719:44 true, 719:44 false |
| `rsk_kernel::arch::cm4::pins_enter` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::usage_fault` | 2 | 0 | 749:24 true, 749:24 false |
| `rsk_kernel::arch::cm4::usage_fault::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::consumed_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry_record` | 6 | 3 | 404:8 true, 407:8 true, 410:8 true |
| `rsk_kernel::arch::cm4::pins_running` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::pushed_kinds` | 8 | 5 | 380:18 true, 385:22 false, 387:12 false |
| `rsk_kernel::arch::cm4::trace_expect` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mirror_events` | 8 | 6 | 353:22 true, 356:26 false |
| `rsk_kernel::arch::cm4::unsafe_module::arch_state` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::fault_push` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::svc_number` | 2 | 1 | 849:12 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_entry` | 4 | 3 | 522:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_stamp` | 10 | 6 | 376:8 true, 376:37 true, 380:8 true, 385:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::faults_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_intake` | 4 | 3 | 397:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::trace_frozen` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::actions_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::events_expect` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_cs_open` | 4 | 2 | 495:8 true, 495:37 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_cs_close` | 6 | 4 | 507:8 true, 507:37 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_response` | 6 | 4 | 355:8 true, 360:16 true |
| `rsk_kernel::arch::cm4::unsafe_module::trace_ram_push` | 2 | 1 | 551:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events_seen_seq` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_calibrate` | 4 | 0 | 417:11 true, 417:11 false, 421:12 true, 421:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::events_push_diag` | 4 | 0 | 474:8 true, 474:8 false, 485:8 true, 485:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats_interrupted` | 2 | 1 | 539:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dwt_cyccnt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::kernel_enter` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::running` | 4 | 3 | 1079:28 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fault_status` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stamp_capture` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::coverage_reset` | 10 | 7 | 188:19 false, 190:20 false, 190:34 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stack_pointers` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stamp_probe_init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::wait_for_interrupt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::boot_info` | 8 | 4 | 715:8 false, 725:12 false, 725:32 false, 728:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats` | 2 | 1 | 303:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events` | 2 | 1 | 713:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::extend` | 2 | 1 | 804:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::faults` | 2 | 0 | 630:8 true, 630:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::actions` | 2 | 0 | 683:8 true, 683:8 false |
| `rsk_kernel::arch::cm4::svc` | 22 | 16 | 588:8 true, 629:12 true, 629:20 true, 629:20 false, 635:12 false, 646:8 true |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::boot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::plan::find_level` | 4 | 4 |  |
| `rsk_kernel::arch::cm4::plan::plan_frame` | 4 | 2 | 155:8 true, 164:8 true |
| `rsk_kernel::arch::cm4::plan::compare_target` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::entry::apply_core` | 14 | 11 | 577:48 true, 607:12 true, 610:12 true |
| `rsk_kernel::arch::cm4::entry::fault_core` | 4 | 0 | 1266:18 true, 1266:18 false, 1266:31 true, 1266:31 false |
| `rsk_kernel::arch::cm4::entry::raise_core` | 6 | 3 | 890:23 false, 895:17 false, 902:8 false |
| `rsk_kernel::arch::cm4::entry::start_core` | 2 | 1 | 926:8 false |
| `rsk_kernel::arch::cm4::entry::timer_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::arm_compare` | 2 | 1 | 533:21 false |
| `rsk_kernel::arch::cm4::entry::charge_point` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::end_plan_core` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::entry::epilogue_core` | 8 | 5 | 1223:8 false, 1232:22 true, 1239:16 true |
| `rsk_kernel::arch::cm4::entry::dma_write_core` | 24 | 13 | 844:8 false, 844:17 false, 844:36 false, 862:16 false, 863:20 true, 863:36 true, 863:55 true, 863:76 true, 869:20 false, 869:29 false, 871:8 false |
| `rsk_kernel::arch::cm4::entry::irq_entry_core` | 14 | 8 | 990:16 true, 995:16 true, 1002:46 false, 1002:92 false, 1011:16 true, 1070:16 false |
| `rsk_kernel::arch::cm4::entry::watchdog_reload` | 4 | 0 | 118:8 true, 118:8 false, 123:8 true, 123:8 false |
| `rsk_kernel::arch::cm4::entry::dma_request_core` | 6 | 3 | 800:23 false, 805:17 false, 812:8 false |
| `rsk_kernel::arch::cm4::entry::dma_stop_partition` | 10 | 0 | 746:11 true, 746:11 false, 751:12 true, 751:12 false, 752:16 true, 752:16 false, 755:16 true, 755:16 false, 758:16 true, 758:16 false |
| `rsk_kernel::arch::cm4::entry::Cm4Hw::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_boot` | 6 | 0 | 774:11 true, 774:11 false, 778:12 true, 778:12 false, 781:12 true, 781:12 false |
| `rsk_kernel::arch::cm4::entry::svc_core` | 24 | 18 | 1128:23 false, 1133:23 false, 1158:23 false, 1159:12 false, 1173:23 false, 1179:15 false |
| `rsk_kernel::arch::cm4::entry::boot_core` | 18 | 6 | 637:8 true, 648:8 true, 651:8 true, 654:8 false, 658:8 true, 666:8 false, 683:11 true, 683:11 false, 714:11 true, 714:11 false, 714:21 true, 714:21 false |
| `rsk_kernel::arch::cm4::fault` | 2 | 0 | 692:8 true, 692:8 false |
| `rsk_kernel::arch::cm4::panic` | 2 | 0 | 782:8 true, 782:8 false |
| `rsk_kernel::arch::cm4::start` | 2 | 0 | 445:8 true, 445:8 false |
| `rsk_kernel::arch::cm4::timer` | 4 | 0 | 553:16 true, 553:16 false, 558:8 true, 558:8 false |
| `rsk_kernel::arch::cm4::trace` | 4 | 4 |  |
| `rsk_kernel::arch::cm4::epilogue` | 2 | 1 | 673:8 true |
| `rsk_kernel::arch::cm4::bus_fault` | 2 | 0 | 730:23 true, 730:23 false |
| `rsk_kernel::arch::cm4::irq_entry` | 18 | 16 | 467:37 false, 507:37 false |
| `rsk_kernel::arch::bound::config_crc` | 20 | 20 |  |
| `rsk_kernel::arch::bound::layout_crc` | 6 | 6 |  |
| `rsk_kernel::arch::bound::basepri_for` | 2 | 2 |  |
| `rsk_kernel::arch::bound::encode_level` | 2 | 1 | 55:8 true |
| `rsk_kernel::arch::bound::basepri_for_ceiling` | 2 | 2 |  |
| `rsk_kernel::arch::bound::Crc32::new` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::byte` | 4 | 4 |  |
| `rsk_kernel::arch::bound::Crc32::finish` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_return` | 2 | 1 | 631:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_detail` | 2 | 1 | 733:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::safe_state` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_unlock` | 2 | 1 | 644:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::task_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::basepri_now` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_failed` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_used` | 2 | 1 | 565:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_instant` | 2 | 1 | 741:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_reinit` | 4 | 2 | 703:12 true, 707:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::timer_event` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::next_compare` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_of` | 2 | 0 | 573:12 true, 573:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::release_info` | 2 | 1 | 552:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top_priority` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dma_permitted` | 2 | 1 | 769:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_partition` | 2 | 1 | 749:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::source_masked` | 2 | 1 | 694:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_stop_dma` | 4 | 2 | 714:12 true, 718:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_view` | 2 | 1 | 698:20 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::overrun_continues` | 4 | 0 | 577:12 true, 577:12 false, 579:13 true, 579:13 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_stopped` | 2 | 1 | 765:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::init` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::depth` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::start` | 2 | 1 | 522:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::intake` | 8 | 4 | 668:12 false, 669:16 false, 670:16 false, 671:16 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::select` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::ceiling` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_due` | 2 | 1 | 561:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_seq` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dispatch` | 4 | 2 | 625:12 false, 625:35 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::eligible` | 10 | 6 | 611:12 true, 616:9 false, 617:16 false, 618:16 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_info` | 2 | 1 | 536:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_kind` | 2 | 1 | 729:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_task` | 2 | 1 | 737:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::stack_at` | 2 | 1 | 603:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_lock` | 4 | 2 | 637:12 true, 637:44 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::validate` | 10 | 5 | 496:9 false, 497:16 false, 498:16 false, 499:16 false, 500:16 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_of` | 2 | 1 | 569:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::is_killed` | 2 | 1 | 607:12 false |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::task_entry` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::stack_bottom` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::intake_vector` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::dispatch_vector` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::dma` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::kernel` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::signal` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::binding` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::stack_top` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::now` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::cycles` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::boot_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::boot_expect` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::signal_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::stack_bottom` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::timer_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::intake_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dispatch_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::fp_flush_to_zero` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::task_entry_address` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::signal` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::binding` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::tick_hz` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::irq_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::stack_top` | 0 | 0 |  |
| `rsk_entry::ipsr` | 0 | 0 |  |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate_slots` | 44 | 27 | 411:12 true, 428:22 false, 428:46 false, 428:70 false, 429:21 true, 429:40 false, 429:64 false, 429:74 false, 430:21 false, 431:16 true, 448:36 false, 448:47 false, 448:58 false, 449:29 true, 449:46 false, 449:57 false, 450:24 true |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate_partitions` | 36 | 21 | 364:28 true, 364:65 false, 365:20 true, 365:20 false, 366:69 true, 367:20 true, 367:66 true, 369:31 true, 369:81 true, 370:20 true, 371:18 false, 371:35 false, 371:47 false, 371:61 false, 386:24 true |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate` | 66 | 38 | 241:14 false, 241:24 false, 241:37 false, 241:50 false, 241:60 false, 242:16 false, 242:47 false, 257:22 false, 257:44 false, 258:20 false, 259:51 false, 260:20 false, 261:20 false, 261:41 false, 262:20 false, 263:21 false, 263:53 true, 263:53 false, 264:20 false, 266:16 true, 287:18 false, 287:51 false, 287:71 false, 310:24 true, 310:55 true, 335:16 true, 343:12 true, 346:12 true |
| `rsk_logic::logic::log::Log<_>::get` | 2 | 1 | 147:22 true |
| `rsk_logic::logic::log::Log<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::log::Log<_>::push` | 6 | 2 | 113:12 false, 114:26 true, 127:28 true, 127:28 false |
| `rsk_logic::logic::mpu::Layout<_>::check_apart` | 4 | 3 | 525:16 true |
| `rsk_logic::logic::mpu::Layout<_>::validate_pair` | 42 | 24 | 606:14 false, 606:40 false, 606:65 false, 606:91 false, 609:14 false, 609:48 false, 609:81 false, 609:114 false, 614:12 true, 614:32 true, 628:16 true, 631:18 false, 631:48 false, 642:20 true, 645:20 true, 665:16 true, 668:18 false, 668:48 false |
| `rsk_logic::logic::mpu::Layout<_>::validate_partition` | 48 | 25 | 539:14 false, 539:39 false, 539:63 false, 543:14 false, 543:35 false, 546:12 true, 549:12 true, 552:14 false, 552:60 false, 552:104 false, 553:16 false, 553:61 false, 553:104 false, 558:14 false, 558:44 false, 574:16 true, 577:18 false, 577:49 false, 577:81 false, 578:20 false, 578:60 false, 583:16 true, 589:16 true |
| `rsk_logic::logic::mpu::Layout<_>::validate_kernel_windows` | 8 | 5 | 490:12 true, 501:16 true, 504:16 true |
| `rsk_logic::logic::mpu::Layout<_>::view` | 2 | 2 |  |
| `rsk_logic::logic::mpu::Layout<_>::validate` | 24 | 14 | 435:14 false, 435:48 false, 435:83 false, 438:14 false, 438:69 false, 439:16 false, 443:12 true, 457:16 true, 468:22 false, 468:50 false |
| `rsk_logic::logic::mpu::PartitionLayout::dma_permitted_exec` | 14 | 10 | 286:12 true, 293:12 true, 293:49 true, 318:16 true |
| `rsk_logic::logic::mpu::Region::disabled` | 0 | 0 |  |
| `rsk_logic::logic::mpu::Interval::check_rules` | 6 | 3 | 139:14 false, 139:37 false, 148:9 false |
| `rsk_logic::logic::mpu::Interval::check_disjoint` | 2 | 2 |  |
| `rsk_logic::logic::mpu::pow2_u32` | 2 | 2 |  |
| `rsk_logic::logic::time::Slots<_>::arm` | 0 | 0 |  |
| `rsk_logic::logic::time::Slots<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::time::Slots<_>::next` | 6 | 6 |  |
| `rsk_logic::logic::time::Slots<_>::disarm` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::try_unmask` | 4 | 2 | 474:12 false, 474:25 true |
| `rsk_logic::logic::time::Timing<_>::job_started` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::periodic_due` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::intake_decision` | 8 | 4 | 287:12 true, 294:42 false, 296:19 true, 296:19 false |
| `rsk_logic::logic::time::Timing<_>::advance_periodic` | 4 | 3 | 259:20 true |
| `rsk_logic::logic::time::Timing<_>::deferred_due_clamped` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::new` | 2 | 2 |  |
| `rsk_logic::logic::time::Timing<_>::mask` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::defer` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::accept` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::unmask` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::finish_top` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::make_pending` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::clear_release` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::recompute_ceiling` | 4 | 3 | 875:16 false |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::new` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::kill` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::lock` | 12 | 5 | 773:12 true, 779:15 true, 784:16 true, 784:16 false, 789:12 true, 792:12 true, 800:12 false |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::defer` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::start` | 0 | 0 |  |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::select` | 14 | 13 | 638:53 false |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::unlock` | 6 | 3 | 844:12 true, 848:12 true, 848:37 true |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::end_job` | 4 | 3 | 907:24 true |
| `rsk_logic::logic::sched::Sched<_$C$_$C$_>::complete` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::reset` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::charge` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::report` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::overrun` | 0 | 0 |  |
| `rsk_logic::logic::budget::Budget<_>::remaining` | 2 | 1 | 138:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::ack_reinit` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::arm_budget` | 2 | 1 | 1702:21 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::job_return` | 8 | 5 | 1759:12 true, 1761:27 true, 1761:27 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::push_event` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::serve_slot` | 20 | 15 | 2487:19 false, 2490:23 true, 2492:23 true, 2496:23 false, 2506:20 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::svc_unlock` | 4 | 2 | 1924:12 false, 1932:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::timer_event` | 18 | 16 | 2279:12 true, 2310:60 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::ack_stop_dma` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::budget_event` | 8 | 5 | 2797:12 true, 2800:12 true, 2803:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::deadline_miss` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::mit_violation` | 2 | 0 | 2213:12 true, 2213:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::record_intake` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::apply_response` | 16 | 0 | 2882:12 true, 2882:12 false, 2882:44 true, 2882:44 false, 2888:12 true, 2888:12 false, 2891:16 true, 2891:16 false, 2893:20 true, 2893:20 false, 2895:24 true, 2895:24 false, 2913:12 true, 2913:12 false, 2917:12 true, 2917:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_mit_window` | 4 | 2 | 1852:12 false, 1852:25 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::intake_release` | 16 | 6 | 2089:12 true, 2089:31 true, 2089:48 true, 2105:12 false, 2122:16 true, 2122:16 false, 2122:37 true, 2122:37 false, 2132:29 true, 2132:29 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::stop_partition` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::unmask_instant` | 2 | 1 | 1819:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_top_release` | 2 | 0 | 2951:12 true, 2951:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::deferred_release` | 6 | 0 | 2689:12 true, 2689:12 false, 2689:48 true, 2689:48 false, 2700:12 true, 2700:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::enter_safe_state` | 4 | 4 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::periodic_release` | 10 | 6 | 2567:12 true, 2578:12 false, 2579:16 false, 2580:20 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::boot_check_failed` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::restart_partition` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_partition_jobs` | 4 | 0 | 3238:12 true, 3238:12 false, 3243:16 true, 3243:16 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::kill_partition_jobs` | 6 | 0 | 3098:15 true, 3098:15 false, 3117:16 true, 3117:16 false, 3117:64 true, 3117:64 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::top_holds_nothing_exec` | 4 | 0 | 1796:15 true, 1796:15 false, 1801:16 true, 1801:16 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::clear_partition_releases` | 12 | 0 | 3154:15 true, 3154:15 false, 3165:16 true, 3165:16 false, 3174:20 true, 3174:20 false, 3174:58 true, 3174:58 false, 3190:20 true, 3190:20 false, 3190:58 true, 3190:58 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::new` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::fault` | 4 | 1 | 2850:12 false, 2850:22 true, 2850:22 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::start` | 6 | 5 | 1611:24 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::intake` | 6 | 1 | 1991:12 true, 1991:23 true, 1991:23 false, 1991:50 true, 1991:50 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::record` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::select` | 4 | 3 | 3436:31 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_top` | 2 | 0 | 2984:12 true, 2984:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::kill_at` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::observe` | 2 | 2 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::dispatch` | 2 | 2 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::svc_lock` | 4 | 2 | 1890:12 false, 1898:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::boot_info` | 4 | 2 | 3384:12 false, 3387:12 false |
| `rsk_logic::logic::kernel::fault_prs` | 8 | 4 | 142:8 true, 144:15 true, 144:38 true, 146:15 true |
| `rsk_busfault_rust` | 0 | 0 |  |
| `rsk_default_entry` | 0 | 0 |  |
| `rsk_epilogue_rust` | 0 | 0 |  |
| `rsk_hardfault_entry` | 0 | 0 |  |
| `rsk_irq_entry_rust` | 0 | 0 |  |
| `rsk_memmanage_rust` | 0 | 0 |  |
| `rsk_nmi_entry` | 0 | 0 |  |
| `rsk_svc_rust` | 0 | 0 |  |
| `rsk_timer_rust` | 0 | 0 |  |
| `rsk_usagefault_rust` | 0 | 0 |  |
| `rsk_entry::interrupts` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::peek` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::peek` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::watchdog_feed` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::safe_state_idle` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::semihosting_exit` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::debugger_attached` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::first_memmanage` | 0 | 0 |  |
| `rsk_system::report_and_exit` | 44 | 31 | 254:19 true, 256:57 true, 284:23 false, 284:38 false, 284:53 false, 284:64 false, 284:74 false, 284:84 false, 284:100 false, 284:110 false, 285:12 false, 285:22 false, 285:35 false |
| `rsk_system::print_kv` | 2 | 2 |  |
| `rsk_system::write_num` | 6 | 6 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_detail` | 2 | 1 | 733:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_instant` | 2 | 1 | 741:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_prs` | 2 | 1 | 745:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_kind` | 2 | 1 | 729:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_task` | 2 | 1 | 737:12 false |
| `rsk_logic::logic::log::Log<_>::get` | 2 | 1 | 147:22 true |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
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
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 237:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_exec` | 10 | 8 | 438:8 true, 446:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::action_push` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::build_frame` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::events_push` | 22 | 10 | 762:8 true, 780:20 false, 780:33 true, 780:61 true, 780:61 false, 784:21 true, 785:8 false, 785:21 true, 785:40 true, 785:40 false, 785:53 true, 785:53 false |
| `rsk_kernel::arch::cm4::unsafe_module::frame_words` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_release` | 10 | 6 | 325:8 true, 328:8 true, 328:23 true, 328:39 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dwt_enable` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::init` | 2 | 0 | 1059:19 true, 1059:19 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::boot_checks` | 32 | 17 | 642:8 true, 645:8 true, 648:8 true, 657:8 true, 661:8 true, 661:40 true, 664:8 true, 667:8 true, 685:12 false, 689:19 false, 696:12 true, 696:24 true, 699:12 true, 703:8 false, 703:29 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_program` | 6 | 3 | 783:8 false, 786:8 false, 786:24 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_enable` | 2 | 1 | 437:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::primask_set` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::rasr_encode` | 8 | 8 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::rbar_encode` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_list_off` | 2 | 0 | 861:8 true, 861:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_disable` | 2 | 1 | 443:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_ack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_arm` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mmio_write_u8` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::primask_clear` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::write_basepri` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fpu_set_access` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_disdefwbuf` | 2 | 0 | 483:32 true, 483:32 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::watchdog_start` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::ppi_disable_all` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_set_pending` | 2 | 1 | 449:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::partition_reinit` | 12 | 0 | 251:19 true, 251:19 false, 253:20 true, 253:20 false, 253:34 true, 253:34 false, 256:20 true, 256:20 false, 260:27 true, 260:27 false, 260:39 true, 260:39 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_set_priority` | 2 | 0 | 461:12 true, 461:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::clear_fault_status` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_clear_pending` | 2 | 1 | 455:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_nonbasethrdena` | 2 | 0 | 477:30 true, 477:30 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fpu_configure_eager` | 2 | 0 | 545:33 true, 545:33 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mpu_enable_background` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_prigroup_all_preempt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::priority_bits_implemented` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_kernel_exception_priorities` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::peripheral` | 4 | 2 | 92:9 false, 92:23 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::kernel_access_ok` | 2 | 0 | 124:9 true, 124:9 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::ppb_known` | 24 | 0 | 102:15 true, 102:15 false, 103:16 true, 103:16 false, 109:9 true, 109:9 false, 110:18 true, 110:18 false, 110:36 true, 110:36 false, 111:21 true, 111:21 false, 111:39 true, 111:39 false, 112:21 true, 112:21 false, 112:39 true, 112:39 false, 113:21 true, 113:21 false, 113:39 true, 113:39 false, 114:21 true, 114:21 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::irq_of` | 4 | 2 | 429:8 true, 429:18 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::barrier` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_stop` | 2 | 0 | 844:8 true, 844:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mpu_load` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::ack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::arm` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_write` | 2 | 1 | 854:8 false |
| `rsk_kernel::arch::cm4::trace_tag` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::u32` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::u64` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::boot_checks` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::apply` | 2 | 1 | 443:20 true |
| `rsk_kernel::arch::placeholder_view` | 0 | 0 |  |
| `rsk_kernel::arch::placeholder_kernel` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::hard_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mem_manage` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::usage_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::usage_fault::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::consumed_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry_record` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::pushed_kinds` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mirror_events` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::peek` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::ResourceCell<T>::with` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::_unused` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::read_basepri` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::kernel_byte_access_ok` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::plausible` | 0 | 0 |  |
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
| `rsk_kernel::arch::cm4::entry::arm_compare` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::charge_point` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::end_plan_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::epilogue_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_write_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::irq_entry_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_request_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_stop_partition` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_boot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::svc_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::boot_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::panic` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::start` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::timer` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::epilogue` | 0 | 0 |  |
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
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::next_compare` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_of` | 0 | 0 |  |
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
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::overrun_continues` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_stopped` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::init` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::depth` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::start` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::intake` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::select` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::ceiling` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_due` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_prs` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_seq` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::started` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dispatch` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::eligible` | 0 | 0 |  |
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
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dispatch_vector` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::fp_flush_to_zero` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::task_entry_address` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::signal` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::binding` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::tick_hz` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::dma_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::irq_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::SysTables<S>$u20$as$u20$rsk_kernel::arch::cm4::entry::Tables>::stack_top` | 0 | 0 |  |

## Structural coverage of the Kernel on the Target (task 19.5, R53.3)

the Kernel on the Target: lines 68.3 % (3021 of 4426), branch outcomes 55.3 % (718 of 1298), functions 75.1 % (280 of 373) of the Kernel sources

| File | Lines | Branch outcomes | Functions |
|---|---|---|---|
| apps/p1-demo/part0/src/lib.rs | 100.0 % (42 of 42) | 90.0 % (9 of 10) | 100.0 % (12 of 12) |
| apps/p1-demo/part1/src/lib.rs | 100.0 % (47 of 47) | 92.9 % (13 of 14) | 100.0 % (8 of 8) |
| apps/p1-demo/part2/src/lib.rs | 83.8 % (62 of 74) | 50.0 % (7 of 14) | 87.5 % (14 of 16) |
| apps/p1-demo/system/src/lib.rs | 98.2 % (107 of 109) | 75.0 % (39 of 52) | 100.0 % (3 of 3) |
| crates/rsk-entry/src/lib.rs | 57.9 % (44 of 76) | 62.5 % (15 of 24) | 35.7 % (5 of 14) |
| crates/rsk-kernel/src/arch/bound.rs | 91.7 % (297 of 324) | 64.2 % (86 of 134) | 94.9 % (56 of 59) |
| crates/rsk-kernel/src/arch/cm4/entry.rs | 65.8 % (310 of 471) | 49.3 % (72 of 146) | 82.6 % (19 of 23) |
| crates/rsk-kernel/src/arch/cm4/mod.rs | 64.8 % (247 of 381) | 58.0 % (51 of 88) | 72.1 % (31 of 43) |
| crates/rsk-kernel/src/arch/cm4/plan.rs | 51.1 % (24 of 47) | 80.0 % (8 of 10) | 75.0 % (3 of 4) |
| crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs | 52.8 % (205 of 388) | 39.6 % (57 of 144) | 54.8 % (34 of 62) |
| crates/rsk-kernel/src/arch/cm4/unsafe_module/mod.rs | 64.3 % (214 of 333) | 52.0 % (52 of 100) | 66.7 % (24 of 36) |
| crates/rsk-kernel/src/arch/mod.rs | 0.0 % (0 of 47) | - (0 of 0) | 0.0 % (0 of 2) |
| crates/rsk-logic/src/logic/budget.rs | 86.4 % (19 of 22) | 50.0 % (1 of 2) | 83.3 % (5 of 6) |
| crates/rsk-logic/src/logic/kernel.rs | 56.8 % (677 of 1191) | 44.7 % (85 of 190) | 62.5 % (25 of 40) |
| crates/rsk-logic/src/logic/log.rs | 65.9 % (27 of 41) | 37.5 % (3 of 8) | 100.0 % (3 of 3) |
| crates/rsk-logic/src/logic/mod.rs | 74.7 % (115 of 154) | 58.9 % (86 of 146) | 100.0 % (3 of 3) |
| crates/rsk-logic/src/logic/mpu.rs | 87.9 % (233 of 265) | 59.2 % (90 of 152) | 100.0 % (11 of 11) |
| crates/rsk-logic/src/logic/sched.rs | 86.5 % (224 of 259) | 67.5 % (27 of 40) | 84.6 % (11 of 13) |
| crates/rsk-logic/src/logic/time.rs | 81.9 % (127 of 155) | 70.8 % (17 of 24) | 86.7 % (13 of 15) |
| **total** | 68.3 % (3021 of 4426) | 55.3 % (718 of 1298) | 75.1 % (280 of 373) |
