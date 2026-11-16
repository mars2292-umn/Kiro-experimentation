# HIL coverage run (task 19.2, R34.4) over the release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 44536 cycles (695.9 µs) against J_rel 12500 cycles |
| Kernel events | pass | 4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 87579 cycles (1368.4 µs) over 1206913 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9998 stimulus pulses answered (target counted 9998; 11249 rises and 9999 response edges in the capture); latency 460 to 8790 µs, median 1130 µs, 0 later than 15 ms; Δ_stamp observed 24404 cycles (381.3 µs) against the exported bound 8000 cycles EXCEEDED; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 53.9 % of the run, the Kernel 51.9 % over 1286273 entries; runs per Task [18597, 6102, 20995, 9624, 9999, 7704, 1823] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 97360 cycles (1.521 ms, cycle-counter read cost 27 cycles) against the trace pin's longest run 94720 cycles (1.480 ms at 100000 samples/s; 18597 runs for 18000 Jobs): the agent exceeds the trace beyond quantization, within a tenth |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 85 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 54 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 44536 / 695.9 | 17778 | 2^14:3755 2^15:14244 |
| 2 | 8999 | 30836 / 481.8 | 17575 | 2^14:4336 2^15:4663 |
| 3 | 4499 | 32796 / 512.4 | 17337 | 2^14:2 2^15:4497 |
| 5 | 7199 | 44536 / 695.9 | 18034 | 2^14:1957 2^15:5242 |
| 6 | 1799 | 32796 / 512.4 | 18373 | 2^15:1799 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 4.919 | 0 | 0 | 0 |
| 1 | 5999 | 1.364 | 0 | 0 | 0 |
| 2 | 8999 | 9.011 | 0 | 0 | 0 |
| 3 | 4500 | 17.333 | 0 | 0 | 0 |
| 4 | 9998 | 8.867 | 9999 | 10278 | 9506 |
| 5 | 7200 | 13.748 | 0 | 0 | 0 |
| 6 | 1800 | 19.054 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 27 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 97360 / 1.521 (min 44173, mean 52178) | 0 above the Budget; 2^15:18000 |
| Task 1 entry | 5999 | 32975 / 0.515 (min 14020, mean 14562) | 0 above the Budget; 2^14:5988 2^15:11 |
| Task 2 entry | 8999 | 445537 / 6.962 (min 393197, mean 406910) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 4500 | 296050 / 4.626 (min 127966, mean 211189) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9998 | 61418 / 0.960 (min 14452, mean 16600) | 0 above the Budget; 2^14:8994 2^15:1004 |
| Task 5 entry | 7200 | 90299 / 1.411 (min 29392, mean 33977) | 0 above the Budget; 2^15:7200 |
| Task 6 entry | 1800 | 94040 / 1.469 (min 34576, mean 37172) | 0 above the Budget; 2^15:1800 |
| Task 0 on Resource 0 | 18000 | 53653 / 0.838 | |
| Task 0 on Resource 1 | 18000 | 20628 / 0.322 | |
| Task 2 on Resource 0 | 8999 | 360531 / 5.633 | |
| Task 2 on Resource 1 | 17998 | 52508 / 0.820 | |
| Task 5 on Resource 2 | 7200 | 38657 / 0.604 | |
| Task 6 on Resource 2 | 1800 | 20107 / 0.314 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56496 | 12292 / 192.06 | 11233 |
| timer | 47819 | 16664 / 260.38 | 13657 |
| intake | 9999 | 10709 / 167.33 | 9904 |
| lock | 71997 | 1721 / 26.89 | 1719 |
| unlock | 71997 | 8474 / 132.41 | 7522 |
| job_end | 56496 | 9614 / 150.22 | 8501 |
| now | 970865 | 924 / 14.44 | 923 |
| raise | 6000 | 9651 / 150.80 | 9254 |
| dma | 1808 | 3471 / 54.23 | 2442 |
| epilogue | 56496 | 4971 / 77.67 | 2810 |
| view | 74147 | 1795 / 28.05 | 1554 |

Entries that interrupted an executing Job: Task 0: 3000, Task 1: 11, Task 2: 30648, Task 3: 9719, Task 4: 1010, Task 5: 1989, Task 6: 254

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 16484 cycles
- Task 0 at 179940.000 ms, 16276 cycles
- Task 2 at 179945.000 ms, 15604 cycles
- Task 0 at 179950.000 ms, 19164 cycles
- Task 5 at 179950.000 ms, 19164 cycles
- Task 0 at 179960.000 ms, 16392 cycles
- Task 3 at 179960.000 ms, 16392 cycles
- Task 2 at 179965.000 ms, 16724 cycles
- Task 0 at 179970.000 ms, 17196 cycles
- Task 5 at 179975.000 ms, 15680 cycles
- Task 0 at 179980.000 ms, 15640 cycles
- Task 0 at 179990.000 ms, 15244 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 1206913 of them, the longest 87579 cycles (1368.4 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 1024 to 2047 (16.0 to 32.0 µs) | 956203 |
| 2048 to 4095 (32.0 to 64.0 µs) | 73222 |
| 4096 to 8191 (64.0 to 128.0 µs) | 2 |
| 8192 to 16383 (128.0 to 256.0 µs) | 90761 |
| 16384 to 32767 (256.0 to 512.0 µs) | 55069 |
| 32768 to 65535 (512.0 to 1024.0 µs) | 31490 |
| 65536 to 131071 (1024.0 to 2048.0 µs) | 166 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 54164 | 87579 / 1368.4 |
| timer | 27996 | 70437 / 1100.6 |
| job_end | 1095753 | 20686 / 323.2 |
| epilogue | 29000 | 17523 / 273.8 |

## The Kernel's events

4 events mirrored (4 held, 1 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 1396470 | 18597 | 1.480 |
| 1 | 114954 | 6102 | 0.470 |
| 2 | 5946877 | 20995 | 6.400 |
| 3 | 1574091 | 9624 | 3.950 |
| 4 | 214792 | 9999 | 0.750 |
| 5 | 359474 | 7704 | 1.030 |
| 6 | 96078 | 1823 | 1.050 |
| none | 8291804 | 38835 | 8.370 |

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
loop:part0:sensor:hold=85
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=54
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
     Finished in 18.49s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 92 % (13 of 14 branch outcomes), part1.pulse 100 % (6 of 6 branch outcomes), part2.filter 50 % (6 of 12 branch outcomes), part2.sampler 50 % (7 of 14 branch outcomes); 0 below

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
| `part2.filter` | 14 | 12 | 6 | 50 % | pass |
| `part2.filter#cs:stats` | 14 | 12 | 6 | 50 % | pass |
| `part2.sampler` | 14 | 14 | 7 | 50 % | pass |
| `part2.sampler#cs:stats` | 14 | 14 | 7 | 50 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `Reset` | 22 | 15 | 107:35 true, 107:57 false, 111:26 false, 121:35 true, 121:57 false, 126:26 false, 141:8 false |
| `__rustc::rust_begin_unwind` | 2 | 0 | 232:8 true, 232:8 false |
| `rsk_kernel::arch::cm4::hard_fault` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mem_manage` | 6 | 0 | 817:12 true, 817:12 false, 819:41 true, 819:41 false, 824:44 true, 824:44 false |
| `rsk_kernel::arch::cm4::pins_enter` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::safe_state` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::span_close` | 6 | 6 |  |
| `rsk_kernel::arch::cm4::usage_fault` | 2 | 0 | 854:24 true, 854:24 false |
| `rsk_kernel::arch::cm4::usage_fault::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::consumed_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry_record` | 6 | 3 | 415:8 true, 418:8 true, 421:8 true |
| `rsk_kernel::arch::cm4::pins_running` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::pushed_kinds` | 8 | 5 | 391:18 true, 396:22 false, 398:12 false |
| `rsk_kernel::arch::cm4::trace_expect` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::mirror_events` | 8 | 6 | 364:22 true, 367:26 false |
| `rsk_kernel::arch::cm4::unsafe_module::arch_state` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::fault_push` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::span_enter` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::span_leave` | 10 | 8 | 774:19 true, 790:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::span_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::svc_number` | 2 | 1 | 998:12 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_entry` | 4 | 3 | 527:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_stamp` | 10 | 6 | 381:8 true, 381:37 true, 385:8 true, 390:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::faults_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_intake` | 4 | 3 | 402:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::trace_frozen` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::actions_reset` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::events_expect` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_cs_open` | 4 | 2 | 500:8 true, 500:37 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_cs_close` | 6 | 4 | 512:8 true, 512:37 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats_response` | 6 | 4 | 360:8 true, 365:16 true |
| `rsk_kernel::arch::cm4::unsafe_module::trace_ram_push` | 2 | 1 | 556:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events_seen_seq` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_calibrate` | 4 | 0 | 422:11 true, 422:11 false, 426:12 true, 426:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::events_push_diag` | 4 | 0 | 479:8 true, 479:8 false, 490:8 true, 490:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::stats_interrupted` | 2 | 1 | 544:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dwt_cyccnt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::kernel_enter` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::running` | 4 | 3 | 1432:28 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::vectpending` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fault_status` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stamp_capture` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::coverage_reset` | 10 | 7 | 203:19 false, 205:20 false, 205:34 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stack_pointers` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stamp_probe_init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::svc_clear_pending` | 2 | 0 | 715:8 true, 715:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::wait_for_interrupt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::stacking_fault_discard` | 2 | 0 | 730:8 true, 730:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::EndpointStorage<_>::len` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::EndpointStorage<_>::base` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::now` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::boot_info` | 8 | 4 | 1014:8 false, 1024:12 false, 1024:32 false, 1027:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::span` | 2 | 1 | 740:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::stats` | 2 | 1 | 308:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::events` | 2 | 1 | 838:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::extend` | 2 | 1 | 929:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::faults` | 2 | 0 | 635:8 true, 635:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::actions` | 2 | 0 | 803:8 true, 803:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::windows` | 2 | 0 | 702:8 true, 702:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::frame_pc` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc_frame_check` | 2 | 0 | 635:8 true, 635:8 false |
| `rsk_kernel::arch::cm4::svc` | 24 | 17 | 710:12 false, 723:12 true, 723:20 true, 723:20 false, 729:12 false, 740:8 true, 744:27 true |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::svc::{closure}` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::boot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::plan::plan_frame` | 6 | 3 | 315:8 true, 318:8 true, 327:8 true |
| `rsk_kernel::arch::cm4::plan::top_active` | 4 | 4 |  |
| `rsk_kernel::arch::cm4::plan::compare_target` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::plan::frame_in_stack` | 4 | 2 | 294:5 false, 294:22 false |
| `rsk_kernel::arch::cm4::entry::apply_core` | 16 | 13 | 833:48 true, 879:12 true, 887:12 true |
| `rsk_kernel::arch::cm4::entry::fault_core` | 4 | 0 | 3865:18 true, 3865:18 false, 3865:31 true, 3865:31 false |
| `rsk_kernel::arch::cm4::entry::raise_core` | 6 | 3 | 1377:23 false, 1382:17 false, 1389:8 false |
| `rsk_kernel::arch::cm4::entry::start_core` | 2 | 1 | 2295:8 false |
| `rsk_kernel::arch::cm4::entry::timer_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::vectors_ok` | 6 | 4 | 934:20 true, 945:24 true |
| `rsk_kernel::arch::cm4::entry::arm_compare` | 2 | 1 | 717:21 false |
| `rsk_kernel::arch::cm4::entry::channel_pop` | 2 | 0 | 1716:8 true, 1716:8 false |
| `rsk_kernel::arch::cm4::entry::intake_core` | 6 | 4 | 3236:43 false, 3236:72 true |
| `rsk_kernel::arch::cm4::entry::boot_vectors` | 2 | 0 | 1015:11 true, 1015:11 false |
| `rsk_kernel::arch::cm4::entry::channel_push` | 2 | 0 | 1689:8 true, 1689:8 false |
| `rsk_kernel::arch::cm4::entry::charge_point` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::partition_in` | 6 | 0 | 1473:11 true, 1473:11 false, 1478:12 true, 1478:12 false, 1478:37 true, 1478:37 false |
| `rsk_kernel::arch::cm4::entry::psp_in_stack` | 2 | 1 | 2319:8 true |
| `rsk_kernel::arch::cm4::entry::unbound_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::channel_index` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::end_plan_core` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_args` | 32 | 0 | 1917:8 true, 1917:8 false, 1917:28 true, 1917:28 false, 1917:60 true, 1917:60 false, 1917:82 true, 1917:82 false, 1917:103 true, 1917:103 false, 1917:124 true, 1917:124 false, 1921:8 true, 1921:8 false, 1921:44 true, 1921:44 false, 1921:73 true, 1921:73 false, 1926:13 true, 1926:13 false, 1927:20 true, 1927:20 false, 1928:20 true, 1928:20 false, 1929:20 true, 1929:20 false, 1930:20 true, 1930:20 false, 1931:20 true, 1931:20 false, 1936:8 true, 1936:8 false |
| `rsk_kernel::arch::cm4::entry::endpoint_slot` | 4 | 0 | 2036:8 true, 2036:8 false, 2036:41 true, 2036:41 false |
| `rsk_kernel::arch::cm4::entry::epilogue_core` | 24 | 13 | 3705:8 true, 3705:22 true, 3722:8 false, 3731:22 true, 3756:8 true, 3768:12 true, 3768:12 false, 3794:37 true, 3805:16 true, 3816:20 true, 3816:20 false |
| `rsk_kernel::arch::cm4::entry::rearm_compare` | 4 | 3 | 733:34 true |
| `rsk_kernel::arch::cm4::entry::clear_channels` | 2 | 0 | 1506:11 true, 1506:11 false |
| `rsk_kernel::arch::cm4::entry::dma_windows_ok` | 8 | 5 | 1239:12 true, 1242:12 false, 1242:29 true |
| `rsk_kernel::arch::cm4::entry::dma_write_core` | 24 | 13 | 1331:8 false, 1331:17 false, 1331:36 false, 1349:16 false, 1350:20 true, 1350:36 true, 1350:55 true, 1350:76 true, 1356:20 false, 1356:29 false, 1358:8 false |
| `rsk_kernel::arch::cm4::entry::irq_entry_core` | 22 | 10 | 3332:8 false, 3332:24 false, 3343:16 true, 3367:16 false, 3397:8 false, 3397:40 true, 3397:56 true, 3397:56 false, 3397:81 true, 3397:81 false, 3409:12 true, 3409:12 false |
| `rsk_kernel::arch::cm4::entry::rejection_core` | 4 | 0 | 1883:10 true, 1883:10 false, 1883:39 true, 1883:39 false |
| `rsk_kernel::arch::cm4::entry::svc_check_core` | 6 | 3 | 2369:8 true, 2369:31 true, 2369:66 false |
| `rsk_kernel::arch::cm4::entry::violation_core` | 34 | 0 | 1822:8 true, 1822:8 false, 1825:12 true, 1825:12 false, 1825:32 true, 1825:32 false, 1825:64 true, 1825:64 false, 1825:86 true, 1825:86 false, 1830:12 true, 1830:12 false, 1830:41 true, 1830:41 false, 1843:8 true, 1843:8 false, 1843:27 true, 1843:27 false, 1843:47 true, 1843:47 false, 1843:79 true, 1843:79 false, 1843:101 true, 1843:101 false, 1843:122 true, 1843:122 false, 1848:8 true, 1848:8 false, 1848:37 true, 1848:37 false, 1848:66 true, 1848:66 false, 1848:82 true, 1848:82 false |
| `rsk_kernel::arch::cm4::entry::copy_in_checked` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::endpoint_reinit` | 12 | 0 | 1601:11 true, 1601:11 false, 1601:20 true, 1601:20 false, 1619:12 true, 1619:12 false, 1625:19 true, 1625:19 false, 1635:20 true, 1635:20 false, 1635:45 true, 1635:45 false |
| `rsk_kernel::arch::cm4::entry::watchdog_reload` | 4 | 0 | 148:8 true, 148:8 false, 153:8 true, 153:8 false |
| `rsk_kernel::arch::cm4::entry::copy_out_checked` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_request_core` | 6 | 3 | 1287:23 false, 1292:17 false, 1299:8 false |
| `rsk_kernel::arch::cm4::entry::svc_service_core` | 26 | 13 | 3499:23 false, 3500:12 false, 3514:23 false, 3520:15 true, 3534:15 true, 3534:37 true, 3540:23 true, 3540:23 false, 3546:12 true, 3546:12 false, 3546:34 true, 3546:34 false, 3553:15 false |
| `rsk_kernel::arch::cm4::entry::dispatch_job_core` | 12 | 7 | 2956:38 false, 2956:84 false, 2975:8 true, 2999:8 true, 3009:8 true |
| `rsk_kernel::arch::cm4::entry::dma_stop_partition` | 10 | 0 | 1201:11 true, 1201:11 false, 1207:12 true, 1207:12 false, 1208:16 true, 1208:16 false, 1211:16 true, 1211:16 false, 1214:16 true, 1214:16 false |
| `rsk_kernel::arch::cm4::entry::endpoint_recv_core` | 4 | 0 | 2232:8 true, 2232:8 false, 2252:16 true, 2252:16 false |
| `rsk_kernel::arch::cm4::entry::endpoint_send_core` | 4 | 0 | 2149:8 true, 2149:8 false, 2194:8 true, 2194:8 false |
| `rsk_kernel::arch::cm4::entry::endpoint_reinit_one` | 4 | 0 | 1762:11 true, 1762:11 false, 1779:11 true, 1779:11 false |
| `rsk_kernel::arch::cm4::entry::dispatch_vector_core` | 30 | 11 | 3116:8 false, 3116:24 false, 3116:49 true, 3142:8 false, 3142:24 true, 3147:15 true, 3147:15 false, 3151:16 true, 3151:16 false, 3151:30 true, 3151:30 false, 3151:56 true, 3151:56 false, 3156:12 true, 3156:12 false, 3170:8 true, 3170:18 true, 3186:43 false, 3186:72 true |
| `rsk_kernel::arch::cm4::entry::fault_frame_readable` | 4 | 0 | 2388:8 true, 2388:8 false, 2391:8 true, 2391:8 false |
| `rsk_kernel::arch::cm4::entry::stack_violation_core` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::svc_of_ended_job_core` | 2 | 1 | 2338:8 true |
| `rsk_kernel::arch::cm4::entry::Cm4Hw::new` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::entry::dma_boot` | 6 | 0 | 1257:11 true, 1257:11 false, 1261:12 true, 1261:12 false, 1264:12 true, 1264:12 false |
| `rsk_kernel::arch::cm4::entry::svc_core` | 12 | 9 | 3610:36 true, 3614:23 false, 3619:23 false |
| `rsk_kernel::arch::cm4::entry::boot_core` | 22 | 10 | 1082:8 true, 1113:8 true, 1116:8 true, 1119:8 false, 1122:8 true, 1126:8 true, 1134:8 true, 1142:8 false, 1160:11 true, 1160:11 false, 1160:21 true, 1160:21 false |
| `rsk_kernel::arch::cm4::entry::role_slot` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::fault` | 2 | 0 | 794:8 true, 794:8 false |
| `rsk_kernel::arch::cm4::panic` | 2 | 0 | 887:8 true, 887:8 false |
| `rsk_kernel::arch::cm4::start` | 2 | 0 | 458:8 true, 458:8 false |
| `rsk_kernel::arch::cm4::timer` | 4 | 0 | 595:16 true, 595:16 false, 600:8 true, 600:8 false |
| `rsk_kernel::arch::cm4::trace` | 4 | 4 |  |
| `rsk_kernel::arch::cm4::epilogue` | 2 | 1 | 771:8 true |
| `rsk_kernel::arch::cm4::fault_pc` | 2 | 0 | 648:8 true, 648:8 false |
| `rsk_kernel::arch::cm4::bus_fault` | 2 | 0 | 835:23 true, 835:23 false |
| `rsk_kernel::arch::cm4::irq_entry` | 20 | 18 | 482:37 false, 522:37 false |
| `rsk_kernel::arch::bound::config_crc` | 20 | 20 |  |
| `rsk_kernel::arch::bound::layout_crc` | 8 | 8 |  |
| `rsk_kernel::arch::bound::basepri_for` | 2 | 2 |  |
| `rsk_kernel::arch::bound::encode_level` | 2 | 1 | 55:8 true |
| `rsk_kernel::arch::bound::basepri_for_ceiling` | 2 | 2 |  |
| `rsk_kernel::arch::bound::Crc32::new` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::byte` | 4 | 4 |  |
| `rsk_kernel::arch::bound::Crc32::finish` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_return` | 2 | 1 | 799:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_detail` | 2 | 1 | 930:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::safe_state` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_unlock` | 2 | 1 | 824:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::task_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::basepri_now` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_failed` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_used` | 2 | 1 | 728:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_instant` | 2 | 1 | 938:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_reinit` | 4 | 2 | 900:12 true, 904:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::timer_event` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::in_ro_window` | 2 | 1 | 976:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::next_compare` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_of` | 2 | 0 | 736:12 true, 736:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::record_event` | 2 | 0 | 996:12 true, 996:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::release_info` | 2 | 1 | 715:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top_priority` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dma_permitted` | 2 | 1 | 966:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_partition` | 2 | 1 | 946:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::source_masked` | 2 | 1 | 891:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::take_stop_dma` | 4 | 2 | 911:12 true, 915:12 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_view` | 2 | 1 | 895:20 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::config_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::layout_checksum` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_count` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::message_permitted` | 2 | 0 | 986:12 true, 986:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::overrun_continues` | 4 | 0 | 740:12 true, 740:12 false, 742:13 true, 742:13 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::partition_stopped` | 2 | 1 | 962:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::top` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::init` | 2 | 2 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::depth` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::start` | 2 | 1 | 685:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::intake` | 10 | 5 | 856:12 false, 857:16 false, 858:16 false, 859:16 false, 861:31 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::settle` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::ceiling` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_due` | 2 | 1 | 724:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_seq` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::dispatch` | 4 | 2 | 788:12 false, 788:35 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::eligible` | 10 | 6 | 774:12 true, 779:9 false, 780:16 false, 781:16 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::fault_of` | 2 | 0 | 1002:12 true, 1002:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::job_info` | 2 | 1 | 699:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_kind` | 2 | 1 | 926:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_task` | 2 | 1 | 934:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::stack_at` | 2 | 1 | 766:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::svc_lock` | 4 | 2 | 805:12 true, 805:44 true |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::validate` | 10 | 5 | 659:9 false, 660:16 false, 661:16 false, 662:16 false, 663:16 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::boot_info` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::budget_of` | 2 | 1 | 732:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::is_killed` | 2 | 1 | 770:12 false |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::task_entry` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::stack_bottom` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::intake_vector` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::dispatch_vector` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::endpoint_storage` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::dma` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::kernel` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::signal` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::binding` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::endpoint` | 0 | 0 |  |
| `_<rsk_system::Sys$u20$as$u20$rsk_kernel::arch::System>::stack_top` | 0 | 0 |  |
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
| `rsk_logic::logic::Config<_$C$_$C$_>::validate_slots` | 44 | 27 | 412:12 true, 429:22 false, 429:46 false, 429:70 false, 430:21 true, 430:40 false, 430:64 false, 430:74 false, 431:21 false, 432:16 true, 449:36 false, 449:47 false, 449:58 false, 450:29 true, 450:46 false, 450:57 false, 451:24 true |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate_partitions` | 36 | 21 | 365:28 true, 365:65 false, 366:20 true, 366:20 false, 367:69 true, 368:20 true, 368:66 true, 370:31 true, 370:81 true, 371:20 true, 372:18 false, 372:35 false, 372:47 false, 372:61 false, 387:24 true |
| `rsk_logic::logic::Config<_$C$_$C$_>::validate` | 66 | 38 | 242:14 false, 242:24 false, 242:37 false, 242:50 false, 242:60 false, 243:16 false, 243:47 false, 258:22 false, 258:44 false, 259:20 false, 260:51 false, 261:20 false, 262:20 false, 262:41 false, 263:20 false, 264:21 false, 264:53 true, 264:53 false, 265:20 false, 267:16 true, 288:18 false, 288:51 false, 288:71 false, 311:24 true, 311:55 true, 336:16 true, 344:12 true, 347:12 true |
| `rsk_logic::logic::log::Log<_>::get` | 2 | 1 | 153:22 true |
| `rsk_logic::logic::log::Log<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::log::Log<_>::push` | 6 | 2 | 119:12 false, 120:26 true, 133:28 true, 133:28 false |
| `rsk_logic::logic::mpu::Layout<_>::check_apart` | 4 | 3 | 593:16 true |
| `rsk_logic::logic::mpu::Layout<_>::validate_pair` | 42 | 24 | 690:14 false, 690:40 false, 690:65 false, 690:91 false, 693:14 false, 693:48 false, 693:81 false, 693:114 false, 698:12 true, 698:32 true, 712:16 true, 715:18 false, 715:48 false, 726:20 true, 729:20 true, 749:16 true, 752:18 false, 752:48 false |
| `rsk_logic::logic::mpu::Layout<_>::validate_partition` | 52 | 28 | 607:14 false, 607:39 false, 607:63 false, 611:14 false, 611:35 false, 614:12 true, 617:12 true, 620:14 false, 620:60 false, 620:104 false, 621:16 false, 621:61 false, 621:104 false, 626:14 false, 626:44 false, 643:16 true, 656:20 true, 661:18 false, 661:49 false, 661:81 false, 662:20 false, 662:60 false, 667:16 true, 673:16 true |
| `rsk_logic::logic::mpu::Layout<_>::validate_kernel_windows` | 8 | 5 | 558:12 true, 569:16 true, 572:16 true |
| `rsk_logic::logic::mpu::Layout<_>::view` | 2 | 2 |  |
| `rsk_logic::logic::mpu::Layout<_>::validate` | 24 | 14 | 503:14 false, 503:48 false, 503:83 false, 506:14 false, 506:69 false, 507:16 false, 511:12 true, 525:16 true, 536:22 false, 536:50 false |
| `rsk_logic::logic::mpu::PartitionLayout::in_ro_window_exec` | 8 | 4 | 406:15 false, 413:16 false, 415:20 false, 415:53 false |
| `rsk_logic::logic::mpu::PartitionLayout::dma_permitted_exec` | 14 | 10 | 318:12 true, 325:12 true, 325:49 true, 350:16 true |
| `rsk_logic::logic::mpu::PartitionLayout::message_permitted_exec` | 4 | 0 | 302:12 true, 302:12 false, 309:9 true, 309:9 false |
| `rsk_logic::logic::mpu::Region::disabled` | 0 | 0 |  |
| `rsk_logic::logic::mpu::Interval::check_rules` | 6 | 3 | 139:14 false, 139:37 false, 148:9 false |
| `rsk_logic::logic::mpu::Interval::check_disjoint` | 2 | 2 |  |
| `rsk_logic::logic::mpu::pow2_u32` | 2 | 2 |  |
| `rsk_logic::logic::time::Slots<_>::arm` | 0 | 0 |  |
| `rsk_logic::logic::time::Slots<_>::new` | 0 | 0 |  |
| `rsk_logic::logic::time::Slots<_>::next` | 6 | 6 |  |
| `rsk_logic::logic::time::Slots<_>::disarm` | 0 | 0 |  |
| `rsk_logic::logic::time::Timing<_>::try_unmask` | 4 | 2 | 488:12 false, 488:25 true |
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
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::arm_budget` | 2 | 1 | 1913:21 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::job_return` | 10 | 6 | 1996:12 true, 1998:27 true, 1998:27 false, 2003:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::no_history` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::push_event` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::serve_slot` | 20 | 15 | 2787:19 false, 2790:23 true, 2792:23 true, 2796:23 false, 2806:20 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::svc_unlock` | 4 | 2 | 2175:12 false, 2187:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::timer_event` | 18 | 16 | 2575:12 true, 2607:60 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::ack_stop_dma` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::budget_event` | 8 | 4 | 3110:12 true, 3110:37 true, 3113:12 true, 3116:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::record_event` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::deadline_miss` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::mit_violation` | 2 | 0 | 2505:12 true, 2505:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::apply_response` | 18 | 0 | 3234:12 true, 3234:12 false, 3234:44 true, 3234:44 false, 3240:12 true, 3240:12 false, 3243:16 true, 3243:16 false, 3245:20 true, 3245:20 false, 3247:24 true, 3247:24 false, 3269:28 true, 3269:28 false, 3298:12 true, 3298:12 false, 3302:12 true, 3302:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_mit_window` | 4 | 2 | 2097:12 false, 2097:25 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::intake_release` | 16 | 6 | 2349:12 true, 2349:31 true, 2349:48 true, 2369:12 false, 2394:16 true, 2394:16 false, 2394:37 true, 2394:37 false, 2404:29 true, 2404:29 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::stop_partition` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::unmask_instant` | 2 | 1 | 2055:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_top_release` | 4 | 0 | 3353:12 true, 3353:12 false, 3356:12 true, 3356:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::deferred_release` | 6 | 0 | 2996:12 true, 2996:12 false, 2996:48 true, 2996:48 false, 3007:12 true, 3007:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::enter_safe_state` | 4 | 4 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::periodic_release` | 10 | 6 | 2869:12 true, 2880:12 false, 2881:16 false, 2882:20 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::boot_check_failed` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::restart_partition` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_partition_jobs` | 4 | 0 | 3660:12 true, 3660:12 false, 3665:16 true, 3665:16 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::kill_partition_jobs` | 6 | 0 | 3518:15 true, 3518:15 false, 3537:16 true, 3537:16 false, 3537:64 true, 3537:64 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::top_holds_nothing_exec` | 4 | 0 | 2032:15 true, 2032:15 false, 2037:16 true, 2037:16 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::clear_partition_releases` | 12 | 0 | 3575:15 true, 3575:15 false, 3586:16 true, 3586:16 false, 3595:20 true, 3595:20 false, 3595:58 true, 3595:58 false, 3611:20 true, 3611:20 false, 3611:58 true, 3611:58 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::new` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::fault` | 4 | 1 | 3167:12 false, 3167:22 true, 3167:22 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::start` | 6 | 5 | 1784:24 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::intake` | 6 | 1 | 2247:12 true, 2247:23 true, 2247:23 false, 2247:50 true, 2247:50 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::settle` | 18 | 10 | 3934:41 true, 3970:20 false, 3970:36 false, 3970:56 true, 3971:24 true, 3971:24 false, 3971:69 true, 3971:69 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::end_top` | 2 | 0 | 3388:12 true, 3388:12 false |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::kill_at` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::observe` | 2 | 2 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::dispatch` | 2 | 2 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::fault_of` | 0 | 0 |  |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::svc_lock` | 4 | 2 | 2136:12 false, 2148:12 true |
| `rsk_logic::logic::kernel::Kernel<_$C$_$C$_$C$_$C$_$C$_>::boot_info` | 4 | 2 | 3814:12 false, 3817:12 false |
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
| `rsk_system::report_and_exit` | 46 | 32 | 383:19 true, 385:57 true, 438:23 false, 438:38 false, 438:53 false, 438:64 false, 438:74 false, 438:84 false, 438:100 false, 438:110 false, 439:12 false, 439:22 false, 439:35 false, 439:45 false |
| `rsk_system::print_kv` | 2 | 2 |  |
| `rsk_system::write_num` | 6 | 6 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_detail` | 2 | 1 | 930:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_instant` | 2 | 1 | 938:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_len` | 0 | 0 |  |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_prs` | 2 | 1 | 942:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_kind` | 2 | 1 | 926:12 false |
| `_<rsk_kernel::arch::bound::Bound<_$C$_$C$_$C$_$C$_$C$_>$u20$as$u20$rsk_kernel::arch::bound::KernelOps>::log_task` | 2 | 1 | 934:12 false |
| `rsk_logic::logic::log::Log<_>::get` | 2 | 1 | 153:22 true |
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
| `p1_demo_part2::sampler` | 2 | 1 | 518:8 false |
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
| `p1_demo_part1::log` | 6 | 5 | 418:72 false |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
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
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 385:80 false |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_exec` | 10 | 8 | 443:8 true, 451:8 true |
| `rsk_kernel::arch::cm4::unsafe_module::action_push` | 2 | 0 | 822:8 true, 822:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::build_frame` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::events_push` | 22 | 10 | 887:8 true, 905:20 false, 905:33 true, 905:61 true, 905:61 false, 909:21 true, 910:8 false, 910:21 true, 910:40 true, 910:40 false, 910:53 true, 910:53 false |
| `rsk_kernel::arch::cm4::unsafe_module::frame_words` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::stats_release` | 10 | 6 | 330:8 true, 333:8 true, 333:23 true, 333:39 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dwt_enable` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::trace_pins::init` | 2 | 0 | 1412:19 true, 1412:19 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::boot_checks` | 34 | 18 | 936:8 true, 939:8 true, 942:8 true, 951:8 true, 955:8 true, 955:40 true, 958:8 true, 961:8 true, 979:12 false, 983:19 false, 990:12 true, 990:24 true, 993:12 true, 998:12 true, 1002:8 false, 1002:29 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_program` | 6 | 3 | 1084:8 false, 1087:8 false, 1087:24 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_enable` | 2 | 1 | 646:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::primask_set` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::rasr_encode` | 8 | 8 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::rbar_encode` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_list_off` | 2 | 0 | 1162:8 true, 1162:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_disable` | 2 | 1 | 652:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_ack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_arm` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mmio_write_u8` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::primask_clear` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::write_basepri` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fpu_set_access` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_disdefwbuf` | 2 | 0 | 692:32 true, 692:32 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_rearm` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::watchdog_start` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::ppi_disable_all` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase_cancel` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_set_pending` | 2 | 1 | 658:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::partition_reinit` | 12 | 0 | 266:19 true, 266:19 false, 268:20 true, 268:20 false, 268:34 true, 268:34 false, 271:20 true, 271:20 false, 275:27 true, 275:27 false, 275:39 true, 275:39 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_set_priority` | 2 | 0 | 670:12 true, 670:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::clear_fault_status` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::nvic_clear_pending` | 2 | 1 | 664:12 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_nonbasethrdena` | 2 | 0 | 686:30 true, 686:30 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::fpu_configure_eager` | 2 | 0 | 839:33 true, 839:33 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mpu_enable_background` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_prigroup_all_preempt` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::priority_bits_implemented` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::set_kernel_exception_priorities` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::peripheral` | 4 | 2 | 100:9 false, 100:23 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::kernel_access_ok` | 2 | 0 | 132:9 true, 132:9 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::addr::ppb_known` | 24 | 0 | 110:15 true, 110:15 false, 111:16 true, 111:16 false, 117:9 true, 117:9 false, 118:18 true, 118:18 false, 118:36 true, 118:36 false, 119:21 true, 119:21 false, 119:39 true, 119:39 false, 120:21 true, 120:21 false, 120:39 true, 120:39 false, 121:21 true, 121:21 false, 121:39 true, 121:39 false, 122:21 true, 122:21 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::irq_of` | 4 | 2 | 474:8 true, 474:18 true |
| `rsk_kernel::arch::cm4::unsafe_module::hw::barrier` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_stop` | 2 | 0 | 1145:8 true, 1145:8 false |
| `rsk_kernel::arch::cm4::unsafe_module::hw::mpu_load` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::ack` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::arm` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::init` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::rearm` | 2 | 2 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::timebase::cancel` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::hw::dma_write` | 2 | 1 | 1155:8 false |
| `rsk_kernel::arch::cm4::trace_tag` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::u32` | 0 | 0 |  |
| `rsk_kernel::arch::bound::Crc32::u64` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::boot_checks` | 0 | 0 |  |
| `_<rsk_kernel::arch::cm4::entry::Cm4Hw$u20$as$u20$rsk_kernel::arch::cm4::entry::KernelArch>::apply` | 2 | 1 | 606:20 true |
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
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::get` | 0 | 0 |  |
| `rsk_kernel::arch::cm4::unsafe_module::KernelCell<K>::new` | 0 | 0 |  |
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

the Kernel on the Target: lines 61.5 % (3456 of 5617), branch outcomes 50.6 % (834 of 1648), functions 67.7 % (304 of 449) of the Kernel sources

| File | Lines | Branch outcomes | Functions |
|---|---|---|---|
| apps/p1-demo/part0/src/lib.rs | 93.3 % (42 of 45) | 90.0 % (9 of 10) | 92.3 % (12 of 13) |
| apps/p1-demo/part1/src/lib.rs | 100.0 % (47 of 47) | 92.9 % (13 of 14) | 100.0 % (8 of 8) |
| apps/p1-demo/part2/src/lib.rs | 80.5 % (62 of 77) | 50.0 % (7 of 14) | 82.4 % (14 of 17) |
| apps/p1-demo/system/src/lib.rs | 98.2 % (108 of 110) | 74.1 % (40 of 54) | 100.0 % (3 of 3) |
| crates/rsk-entry/src/lib.rs | 58.2 % (46 of 79) | 61.5 % (16 of 26) | 35.7 % (5 of 14) |
| crates/rsk-kernel/src/arch/bound.rs | 86.4 % (318 of 368) | 61.6 % (90 of 146) | 89.1 % (57 of 64) |
| crates/rsk-kernel/src/arch/cm4/entry.rs | 47.9 % (591 of 1233) | 34.7 % (131 of 378) | 50.9 % (29 of 57) |
| crates/rsk-kernel/src/arch/cm4/mod.rs | 62.7 % (274 of 437) | 58.8 % (60 of 102) | 70.0 % (35 of 50) |
| crates/rsk-kernel/src/arch/cm4/plan.rs | 50.0 % (29 of 58) | 68.8 % (11 of 16) | 80.0 % (4 of 5) |
| crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs | 51.2 % (234 of 457) | 41.0 % (64 of 156) | 51.3 % (39 of 76) |
| crates/rsk-kernel/src/arch/cm4/unsafe_module/mod.rs | 63.9 % (251 of 393) | 53.4 % (63 of 118) | 64.3 % (27 of 42) |
| crates/rsk-kernel/src/arch/mod.rs | 0.0 % (0 of 47) | - (0 of 0) | 0.0 % (0 of 2) |
| crates/rsk-logic/src/logic/budget.rs | 86.4 % (19 of 22) | 50.0 % (1 of 2) | 83.3 % (5 of 6) |
| crates/rsk-logic/src/logic/endpoint.rs | 0.0 % (0 of 41) | 0.0 % (0 of 8) | 0.0 % (0 of 4) |
| crates/rsk-logic/src/logic/kernel.rs | 53.4 % (692 of 1297) | 43.9 % (93 of 212) | 58.5 % (24 of 41) |
| crates/rsk-logic/src/logic/log.rs | 65.9 % (27 of 41) | 37.5 % (3 of 8) | 100.0 % (3 of 3) |
| crates/rsk-logic/src/logic/mod.rs | 74.7 % (115 of 154) | 58.9 % (86 of 146) | 100.0 % (3 of 3) |
| crates/rsk-logic/src/logic/mpu.rs | 84.1 % (249 of 296) | 57.7 % (97 of 168) | 92.3 % (12 of 13) |
| crates/rsk-logic/src/logic/sched.rs | 86.5 % (225 of 260) | 71.7 % (33 of 46) | 84.6 % (11 of 13) |
| crates/rsk-logic/src/logic/time.rs | 81.9 % (127 of 155) | 70.8 % (17 of 24) | 86.7 % (13 of 15) |
| **total** | 61.5 % (3456 of 5617) | 50.6 % (834 of 1648) | 67.7 % (304 of 449) |
