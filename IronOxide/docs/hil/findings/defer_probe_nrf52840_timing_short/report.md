# HIL release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.9 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 274 of 275 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | FAIL | the run ended without RSK-OK after 201 s (console attached) |
| release timing (R9.10) | pass | 40479 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 17599 cycles (275.0 µs) against J_rel 10000 cycles |
| Kernel events | FAIL | 20 events mirrored (20 held, 0 of them the post-detach reboot's): overrun of Task 2: 13, deadline_miss of Task 0: 1, deadline_miss of Task 2: 2, release_overlap of Task 2: 2, mit_violation of Task 2: 1, debugger of Task none: 1; the trace froze at event 19 (deadline_miss of Task 0 at 20540.082 ms); unexpected: deadline_miss of Task 0 at 20540.082 ms (detail 0x0) |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 19 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 189 dispatch vector(s) taken at another dispatch stub's entry and deferred; 2184 actions recorded (32 held, 32 of them deferrals: vectors 0x10028, 0x24); the witness build's widened entry was entered and deferred |
| loopback over the run | pass | 9119 stimulus pulses answered (target counted 9119; 11143 rises and 9120 response edges in the capture); latency 290 to 13350 µs, median 2810 µs, 0 later than 15 ms; Δ_stamp observed 15008 cycles (234.5 µs) against the exported bound 8000 cycles EXCEEDED; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 76.8 % of the run, the Kernel 57.7 % over 1904688 entries; runs per Task [16568, 5470, 29149, 4679, 9360, 6611, 1715] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 37807 cycles (0.591 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 37120 cycles (0.580 ms at 100000 samples/s; 16568 runs for 17992 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1376 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3463 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 347 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17991 | 17599 / 275.0 | 8129 | 2^12:3 2^13:14385 2^14:3602 2^15:1 |
| 2 | 8991 | 17599 / 275.0 | 8175 | 2^12:1 2^13:6907 2^14:2082 2^15:1 |
| 3 | 4499 | 15419 / 240.9 | 8312 | 2^13:3406 2^14:1093 |
| 5 | 7199 | 17599 / 275.0 | 8116 | 2^12:22 2^13:4447 2^14:2729 2^15:1 |
| 6 | 1799 | 17599 / 275.0 | 8610 | 2^14:1798 2^15:1 |

Releases recorded: 40479; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 17992 | 12.198 | 0 | 0 | 0 |
| 1 | 5996 | 0.568 | 0 | 0 | 0 |
| 2 | 8992 | 55.355 | 0 | 0 | 0 |
| 3 | 4500 | 9.141 | 0 | 0 | 0 |
| 4 | 9119 | 13.303 | 9120 | 2164 | 1959 |
| 5 | 7200 | 13.105 | 0 | 0 | 0 |
| 6 | 1039 | 59.362 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 17992 | 37807 / 0.591 (min 13079, mean 19452) | 0 above the Budget; 2^14:8604 2^15:9388 |
| Task 1 entry | 5996 | 22935 / 0.358 (min 7443, mean 7835) | 11 above the Budget; 2^13:5786 2^14:199 2^15:11 |
| Task 2 entry | 8992 | 2919174 / 45.612 (min 790533, mean 808574) | 296 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 228565 / 3.571 (min 187197, mean 199472) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9119 | 28014 / 0.438 (min 7500, mean 10250) | 0 above the Budget; 2^13:6298 2^14:2698 2^15:123 |
| Task 5 entry | 7200 | 31808 / 0.497 (min 10294, mean 13612) | 0 above the Budget; 2^14:5341 2^15:1859 |
| Task 6 entry | 1039 | 26873 / 0.420 (min 11069, mean 15089) | 0 above the Budget; 2^14:540 2^15:499 |
| Task 0 on Resource 0 | 17992 | 16238 / 0.254 | |
| Task 0 on Resource 1 | 17992 | 16159 / 0.252 | |
| Task 2 on Resource 0 | 8992 | 777547 / 12.149 | |
| Task 2 on Resource 1 | 17984 | 15998 / 0.250 | |
| Task 5 on Resource 2 | 7200 | 13688 / 0.214 | |
| Task 6 on Resource 2 | 1046 | 8172 / 0.128 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56840 | 2979 / 46.55 | 2657 |
| timer | 66150 | 4724 / 73.81 | 2999 |
| intake | 9120 | 2293 / 35.83 | 2087 |
| lock | 71871 | 460 / 7.19 | 438 |
| unlock | 71206 | 1817 / 28.39 | 1589 |
| job_end | 54838 | 2019 / 31.55 | 1775 |
| now | 13674716 | 242 / 3.78 | 240 |
| raise | 5997 | 2054 / 32.09 | 1931 |
| dma | 7893 | 708 / 11.06 | 515 |
| epilogue | 55600 | 1215 / 18.98 | 705 |
| restart | 665 | 4724 / 73.81 | 4557 |
| enforce | 1067 | 4724 / 73.81 | 3963 |
| view | 78938 | 330 / 5.16 | 293 |

Entries that interrupted an executing Job: Task 0: 12172, Task 1: 221, Task 2: 58971, Task 3: 2216, Task 4: 3062, Task 5: 2854, Task 6: 2181

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 7807 cycles
- Task 2 at 179940.000 ms, 7807 cycles
- Task 0 at 179950.000 ms, 8311 cycles
- Task 5 at 179950.000 ms, 8311 cycles
- Task 0 at 179960.000 ms, 8079 cycles
- Task 2 at 179960.000 ms, 8079 cycles
- Task 3 at 179960.000 ms, 8079 cycles
- Task 0 at 179970.000 ms, 7911 cycles
- Task 5 at 179975.000 ms, 7751 cycles
- Task 0 at 179980.000 ms, 7771 cycles
- Task 2 at 179980.000 ms, 7771 cycles
- Task 0 at 179990.000 ms, 7787 cycles

## The Kernel's events

20 events mirrored (20 held, 0 of them the post-detach reboot's): overrun of Task 2: 13, deadline_miss of Task 0: 1, deadline_miss of Task 2: 2, release_overlap of Task 2: 2, mit_violation of Task 2: 1, debugger of Task none: 1; the trace froze at event 19 (deadline_miss of Task 0 at 20540.082 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 19 | 20540.082 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 481397 | 16568 | 0.580 |
| 1 | 61663 | 5470 | 0.350 |
| 2 | 10459189 | 29149 | 12.760 |
| 3 | 1276144 | 4679 | 3.320 |
| 4 | 136327 | 9360 | 0.360 |
| 5 | 132960 | 6611 | 0.480 |
| 6 | 65837 | 1715 | 0.680 |
| none | 3799982 | 29619 | 6.740 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short_defer_probe/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short_defer_probe/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=17992
sensor=8992
log=4500
pulse=9119
echo=5996
blocked_releases=8992
filter=1163
filter_state_bits=1120403452
sampler=275
dma_ok=275
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1376
loop:part0:sensor:late=3463
loop:part1:spin_ms:spin=0
loop:part1:log:busy=347
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=274
dma_after_stop=0
spim0_enable=7
spim0_shorts=131072
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=157914
  detail=33622
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=157914
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=158418
  detail=33614
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=158418
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=159714
  detail=33622
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=159714
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=160218
  detail=33614
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=160218
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=161514
  detail=33622
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=161514
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=162018
  detail=33614
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=162018
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=163314
  detail=33622
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=163314
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=163818
  detail=33614
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=163818
  detail=6
  prs_lo=0
  prs_hi=0
event=7
  task=0
  at_ms=164040
  detail=0
  prs_lo=134217728
  prs_hi=0
event=9
  task=0
  at_ms=164040
  detail=0
  prs_lo=4096
  prs_hi=0
event=6
  task=4
  at_ms=164161
  detail=36966
  prs_lo=134217728
  prs_hi=0
event=19
  task=4
  at_ms=164161
  detail=6
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
expected_control=18000
expected_sensor=8992
expected_late_jobs=8
late_events_seen=10
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0xdbbe inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1171:17
    Frame 1: report_and_exit @ 0xdba2
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/system/src/lib.rs:344:5
    Frame 2: safe_state @ 0xe140
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/decl/src/nrf52840_timing_short.rs:26:1
    Frame 3: irq_entry<rsk_system::Sys> @ 0xb82e inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:520:9
    Frame 4: rsk_irq_entry_rust @ 0xa8c2
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-entry/src/lib.rs:154:16
    Frame 5: rsk_irq_entry @ 0x154
    Frame 6: UNWIND: Error while checking for exception context. The stack trace will not include the calling frames.
Probe(Other("Overflow error: Attempting to subtract 1 bytes to Register value 0x00000000")) @ 0x154
     Finished in 7.72s
 WARN probe_rs_debug::debug_info: UNWIND: Error while checking for exception context. The stack trace will not include the calling frames.
Probe(Other("Overflow error: Attempting to subtract 1 bytes to Register value 0x00000000"))
Error: Application exit
```
