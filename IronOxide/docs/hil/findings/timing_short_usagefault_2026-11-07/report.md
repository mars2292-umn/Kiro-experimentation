# HIL release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | FAIL | the run ended without RSK-OK after 192 s (console attached) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7255 cycles (113.4 µs) against J_rel 10000 cycles |
| Kernel events | FAIL | 50 events mirrored (50 held, 0 of them the post-detach reboot's): usagefault of Task 0: 1, overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, debugger of Task none: 1; the trace froze at event 49 (usagefault of Task 0 at 168860.008 ms); unexpected: usagefault of Task 0 at 168860.008 ms (detail 0x0) |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 49 and holds the entries leading to it |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10642 rises and 10001 response edges in the capture); latency 80 to 12260 µs, median 2040 µs, 0 later than 15 ms; Δ_stamp observed 6028 cycles (94.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.5 % over 1998203 entries; runs per Task [18303, 6003, 31719, 5004, 10121, 7267, 1827] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22345 cycles (0.349 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s; 18303 runs for 17999 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1384 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 347 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7255 / 113.4 | 3576 | 2^12:16902 2^13:1097 |
| 2 | 8991 | 7255 / 113.4 | 3606 | 2^12:7981 2^13:1010 |
| 3 | 4499 | 7255 / 113.4 | 3752 | 2^12:3550 2^13:949 |
| 5 | 7199 | 7255 / 113.4 | 3636 | 2^12:6250 2^13:949 |
| 6 | 1799 | 7255 / 113.4 | 4062 | 2^12:894 2^13:905 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 17999 | 9.608 | 0 | 0 | 0 |
| 1 | 5998 | 0.271 | 0 | 0 | 0 |
| 2 | 8991 | 54.327 | 0 | 0 | 0 |
| 3 | 4500 | 6.775 | 0 | 0 | 0 |
| 4 | 10000 | 12.288 | 10000 | 2121 | 1943 |
| 5 | 7200 | 11.325 | 0 | 0 | 0 |
| 6 | 1800 | 43.562 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 17999 | 22345 / 0.349 (min 8406, mean 11340) | 0 above the Budget; 2^14:17984 2^15:15 |
| Task 1 entry | 5998 | 6696 / 0.105 (min 2981, mean 3165) | 0 above the Budget; 2^12:5869 2^13:129 |
| Task 2 entry | 8991 | 2948344 / 46.068 (min 781804, mean 786881) | 8 above the Budget; 2^15:8991 |
| Task 3 entry | 4500 | 199267 / 3.114 (min 181427, mean 195217) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 13552 / 0.212 (min 3017, mean 3523) | 0 above the Budget; 2^12:9238 2^13:640 2^14:122 |
| Task 5 entry | 7200 | 13217 / 0.207 (min 5838, mean 6275) | 0 above the Budget; 2^13:6974 2^14:226 |
| Task 6 entry | 1800 | 25570 / 0.400 (min 6635, mean 9044) | 0 above the Budget; 2^13:605 2^14:1193 2^15:2 |
| Task 0 on Resource 0 | 17999 | 7103 / 0.111 | |
| Task 0 on Resource 1 | 17999 | 3868 / 0.060 | |
| Task 2 on Resource 0 | 8991 | 772612 / 12.072 | |
| Task 2 on Resource 1 | 17982 | 3780 / 0.059 | |
| Task 5 on Resource 2 | 7200 | 7125 / 0.111 | |
| Task 6 on Resource 2 | 1800 | 4596 / 0.072 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2847 / 44.48 | 2687 |
| timer | 52625 | 4371 / 68.30 | 2974 |
| intake | 10000 | 2246 / 35.09 | 2068 |
| lock | 71971 | 464 / 7.25 | 461 |
| unlock | 71971 | 1821 / 28.45 | 1608 |
| job_end | 56488 | 2084 / 32.56 | 1799 |
| now | 13919592 | 242 / 3.78 | 240 |
| raise | 5999 | 2100 / 32.81 | 1979 |
| dma | 1808 | 712 / 11.12 | 497 |
| fault | 1 | 2445 / 38.20 | 2445 |
| epilogue | 56489 | 1220 / 19.06 | 726 |
| restart | 1 | 2445 / 38.20 | 2445 |
| enforce | 8 | 2911 / 45.48 | 2864 |
| view | 79934 | 328 / 5.12 | 290 |

Entries that interrupted an executing Job: Task 0: 8407, Task 1: 129, Task 2: 53973, Task 3: 2186, Task 4: 946, Task 5: 375, Task 6: 1225

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3280 cycles
- Task 2 at 179940.000 ms, 3280 cycles
- Task 0 at 179950.000 ms, 3543 cycles
- Task 5 at 179950.000 ms, 3543 cycles
- Task 0 at 179960.000 ms, 3528 cycles
- Task 2 at 179960.000 ms, 3528 cycles
- Task 3 at 179960.000 ms, 3528 cycles
- Task 0 at 179970.000 ms, 3539 cycles
- Task 5 at 179975.000 ms, 3679 cycles
- Task 0 at 179980.000 ms, 3280 cycles
- Task 2 at 179980.000 ms, 3280 cycles
- Task 0 at 179990.000 ms, 3511 cycles

## The Kernel's fault record

Each fault the Kernel took (CFSR, and MMFAR, BFAR, or the stacked PC), oldest first:

```
1 faults recorded (1 held)
   0: cfsr 0x00040000 (INVPC) at 0x00000000
```

## The Kernel's events

50 events mirrored (50 held, 0 of them the post-detach reboot's): usagefault of Task 0: 1, overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, debugger of Task none: 1; the trace froze at event 49 (usagefault of Task 0 at 168860.008 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 49 | 168860.008 | usagefault | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 299697 | 18303 | 0.340 |
| 1 | 23468 | 6003 | 0.100 |
| 2 | 11153628 | 31719 | 12.250 |
| 3 | 1369480 | 5004 | 3.100 |
| 4 | 43354 | 10121 | 0.200 |
| 5 | 61489 | 7267 | 0.200 |
| 6 | 23085 | 1827 | 0.250 |
| none | 5024774 | 33041 | 9.850 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=1113
sensor=556
log=4500
pulse=10000
echo=370
blocked_releases=556
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1384
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=347
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
event=4
  task=0
  at_ms=168860
  detail=0
  prs_lo=4194304
  prs_hi=0
event=16
  task=255
  at_ms=168860
  detail=4
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
late_events_seen=28
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0xd7a2 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1154:17
    Frame 1: report_and_exit @ 0xd786
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/system/src/lib.rs:344:5
    Frame 2: safe_state @ 0xdd24
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/decl/src/nrf52840_timing_short.rs:26:1
    Frame 3: irq_entry<rsk_system::Sys> @ 0xb526 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:518:9
    Frame 4: rsk_irq_entry_rust @ 0xa7b0
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-entry/src/lib.rs:154:16
    Frame 5: rsk_irq_entry @ 0x14c
    Frame 6: UNWIND: Error while checking for exception context. The stack trace will not include the calling frames.
Probe(Other("Overflow error: Attempting to subtract 1 bytes to Register value 0x00000000")) @ 0x14c
     Finished in 7.39s
 WARN probe_rs_debug::debug_info: UNWIND: Error while checking for exception context. The stack trace will not include the calling frames.
Probe(Other("Overflow error: Attempting to subtract 1 bytes to Register value 0x00000000"))
Error: Application exit
```
