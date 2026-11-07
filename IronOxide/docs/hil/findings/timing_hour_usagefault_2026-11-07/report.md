# HIL release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | FAIL | the run ended without RSK-OK after 3671 s (console attached) |
| release timing (R9.10) | pass | 823318 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7578 cycles (118.4 µs) against J_rel 10000 cycles |
| Kernel events | FAIL | 650 events mirrored (512 held, 0 of them the post-detach reboot's): usagefault of Task 0: 1, overrun of Task 2: 85, deadline_miss of Task 2: 170, release_overlap of Task 2: 171, mit_violation of Task 2: 85; the trace froze at event 649 (usagefault of Task 0 at 2232060.008 ms); unexpected: usagefault of Task 0 at 2232060.008 ms (detail 0x0) |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 8 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 649 and holds the entries leading to it |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 203955 rises and 203334 response edges in the capture); latency 80 to 12610 µs, median 2390 µs, 0 later than 15 ms; Δ_stamp observed 6544 cycles (102.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.5 % over 40635711 entries; runs per Task [372044, 122037, 647113, 107247, 204024, 147211, 36853] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21709 cycles (0.339 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21120 cycles (0.330 ms at 100000 samples/s; 372044 runs for 365999 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1384 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3788 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 347 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 7578 / 118.4 | 3535 | 2^12:347499 2^13:18500 |
| 2 | 182822 | 7339 / 114.7 | 3576 | 2^12:165187 2^13:17635 |
| 3 | 91499 | 7339 / 114.7 | 3721 | 2^12:74564 2^13:16935 |
| 5 | 146399 | 7339 / 114.7 | 3605 | 2^12:129420 2^13:16979 |
| 6 | 36599 | 7339 / 114.7 | 4031 | 2^12:20179 2^13:16420 |

Releases recorded: 823318; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 365999 | 9.656 | 0 | 0 | 0 |
| 1 | 121999 | 0.287 | 0 | 0 | 0 |
| 2 | 182822 | 54.355 | 0 | 0 | 0 |
| 3 | 91500 | 7.135 | 0 | 0 | 0 |
| 4 | 203333 | 12.623 | 203334 | 2157 | 1931 |
| 5 | 146400 | 11.425 | 0 | 0 | 0 |
| 6 | 36600 | 43.581 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 365999 | 21709 / 0.339 (min 8486, mean 11389) | 0 above the Budget; 2^14:363969 2^15:2030 |
| Task 1 entry | 121999 | 9740 / 0.152 (min 2957, mean 3109) | 0 above the Budget; 2^12:121266 2^13:732 2^14:1 |
| Task 2 entry | 182822 | 2957834 / 46.216 (min 781724, mean 786905) | 177 above the Budget; 2^15:182822 |
| Task 3 entry | 91500 | 203368 / 3.178 (min 177522, mean 194891) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 13552 / 0.212 (min 3017, mean 3415) | 0 above the Budget; 2^12:191604 2^13:10631 2^14:1098 |
| Task 5 entry | 146400 | 17610 / 0.275 (min 5858, mean 6246) | 0 above the Budget; 2^13:142082 2^14:4268 2^15:50 |
| Task 6 entry | 36600 | 26062 / 0.407 (min 6635, mean 8988) | 0 above the Budget; 2^13:12362 2^14:24226 2^15:12 |
| Task 0 on Resource 0 | 365999 | 7379 / 0.115 | |
| Task 0 on Resource 1 | 365999 | 7512 / 0.117 | |
| Task 2 on Resource 0 | 182822 | 773744 / 12.090 | |
| Task 2 on Resource 1 | 365644 | 6796 / 0.106 | |
| Task 5 on Resource 2 | 146400 | 11853 / 0.185 | |
| Task 6 on Resource 2 | 36600 | 10827 / 0.169 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148659 | 2871 / 44.86 | 2673 |
| timer | 1066764 | 4431 / 69.23 | 2962 |
| intake | 203334 | 2282 / 35.66 | 2057 |
| lock | 1463464 | 464 / 7.25 | 454 |
| unlock | 1463464 | 1841 / 28.77 | 1601 |
| job_end | 1148653 | 2101 / 32.83 | 1791 |
| now | 283022423 | 242 / 3.78 | 240 |
| raise | 121999 | 2101 / 32.83 | 1970 |
| dma | 36608 | 712 / 11.12 | 490 |
| fault | 1 | 2485 / 38.83 | 2485 |
| epilogue | 1148654 | 1219 / 19.05 | 723 |
| restart | 1 | 2485 / 38.83 | 2485 |
| enforce | 177 | 3027 / 47.30 | 2841 |
| view | 1630373 | 328 / 5.12 | 290 |

Entries that interrupted an executing Job: Task 0: 180127, Task 1: 734, Task 2: 1093831, Task 3: 49339, Task 4: 13045, Task 5: 7184, Task 6: 24598

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3300 cycles
- Task 2 at 3659940.000 ms, 3300 cycles
- Task 0 at 3659950.000 ms, 3851 cycles
- Task 5 at 3659950.000 ms, 3851 cycles
- Task 0 at 3659960.000 ms, 6811 cycles
- Task 2 at 3659960.000 ms, 6811 cycles
- Task 3 at 3659960.000 ms, 6811 cycles
- Task 0 at 3659970.000 ms, 3411 cycles
- Task 5 at 3659975.000 ms, 4787 cycles
- Task 0 at 3659980.000 ms, 3280 cycles
- Task 2 at 3659980.000 ms, 3280 cycles
- Task 0 at 3659990.000 ms, 3591 cycles

## The Kernel's fault record

Each fault the Kernel took (CFSR, and MMFAR, BFAR, or the stacked PC), oldest first:

```
1 faults recorded (1 held)
   0: cfsr 0x00040000 (INVPC) at 0x00000000
```

## The Kernel's events

650 events mirrored (512 held, 0 of them the post-detach reboot's): usagefault of Task 0: 1, overrun of Task 2: 85, deadline_miss of Task 2: 170, release_overlap of Task 2: 171, mit_violation of Task 2: 85; the trace froze at event 649 (usagefault of Task 0 at 2232060.008 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 649 | 2232060.008 | usagefault | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6125397 | 372044 | 0.330 |
| 1 | 468484 | 122037 | 0.140 |
| 2 | 226804819 | 647113 | 12.260 |
| 3 | 27830245 | 107247 | 3.130 |
| 4 | 838579 | 204024 | 0.200 |
| 5 | 1242771 | 147211 | 0.260 |
| 6 | 465043 | 36853 | 0.250 |
| none | 102220173 | 666929 | 9.850 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=142793
sensor=71327
log=91500
pulse=203333
echo=47597
blocked_releases=71327
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1384
loop:part0:sensor:late=3788
loop:part1:spin_ms:spin=0
loop:part1:log:busy=347
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=36599
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
event=10
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=366000
expected_sensor=182822
expected_late_jobs=178
late_events_seen=30
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0xd7a0 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1154:17
    Frame 1: report_and_exit @ 0xd78a
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/system/src/lib.rs:344:5
    Frame 2: safe_state @ 0xdd2e
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/decl/src/nrf52840_timing.rs:24:1
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
