# HIL release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | FAIL | the run ended without RSK-OK after 61 s (console attached) |
| release timing (R9.10) | pass | 9233 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 10019 cycles (156.5 µs) against J_rel 10000 cycles |
| Kernel events | FAIL | 2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 1: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 1 at 140.953 ms); unexpected: overrun of Task 1 at 140.953 ms (detail 0x5e8e) |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 1 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 608 actions recorded (32 held, 0 of them deferrals) |
| loopback over the run | pass | 11 stimulus pulses answered (target counted 11; 3369 rises and 2281 response edges in the capture); latency 310 to 12160 µs, median 5600 µs, 0 later than 15 ms; Δ_stamp observed 12432 cycles (194.2 µs) against the exported bound 8000 cycles EXCEEDED; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 87.0 % of the run, the Kernel 63.7 % over 2090 entries; runs per Task [17, 6, 41, 5, 10, 7, 1] |
| agent against the trace (task 19.1) | FAIL | Task 0: the Budget accounting's largest Job execution 39485 cycles (0.617 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 28160 cycles (0.440 ms at 100000 samples/s; 17 runs for 4104 Jobs): the agent exceeds the trace by more than a tenth |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1347 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3356 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 340 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 4104 | 10019 / 156.5 | 7975 | 2^13:3467 2^14:637 |
| 2 | 2051 | 10019 / 156.5 | 8000 | 2^13:1636 2^14:415 |
| 3 | 1026 | 10019 / 156.5 | 8140 | 2^13:817 2^14:209 |
| 5 | 1642 | 9362 / 146.3 | 8032 | 2^12:1 2^13:1033 2^14:608 |
| 6 | 410 | 9362 / 146.3 | 8440 | 2^14:410 |

Releases recorded: 9233; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 4104 | 11.778 | 0 | 0 | 0 |
| 1 | 1368 | 0.774 | 0 | 0 | 0 |
| 2 | 2051 | 55.367 | 0 | 0 | 0 |
| 3 | 1027 | 8.707 | 0 | 0 | 0 |
| 4 | 2249 | 13.013 | 2281 | 2132 | 1974 |
| 5 | 1642 | 12.782 | 0 | 0 | 0 |
| 6 | 205 | 18.632 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 4104 | 39485 / 0.617 (min 13285, mean 19587) | 0 above the Budget; 2^14:1999 2^15:2105 |
| Task 1 entry | 1368 | 31573 / 0.493 (min 7437, mean 9259) | 101 above the Budget; 2^13:1266 2^14:1 2^15:101 |
| Task 2 entry | 2051 | 2897148 / 45.268 (min 790800, mean 808640) | 23 above the Budget; 2^15:2051 |
| Task 3 entry | 1027 | 228486 / 3.570 (min 190056, mean 199338) | 0 above the Budget; 2^15:1027 |
| Task 4 entry | 2249 | 31312 / 0.489 (min 7497, mean 9487) | 0 above the Budget; 2^13:1725 2^14:522 2^15:2 |
| Task 5 entry | 1642 | 31542 / 0.493 (min 10406, mean 14589) | 0 above the Budget; 2^14:1109 2^15:533 |
| Task 6 entry | 205 | 25971 / 0.406 (min 11187, mean 15956) | 0 above the Budget; 2^14:82 2^15:123 |
| Task 0 on Resource 0 | 4104 | 8206 / 0.128 | |
| Task 0 on Resource 1 | 4104 | 15811 / 0.247 | |
| Task 2 on Resource 0 | 2051 | 776810 / 12.138 | |
| Task 2 on Resource 1 | 4102 | 16022 / 0.250 | |
| Task 5 on Resource 2 | 1642 | 8052 / 0.126 | |
| Task 6 on Resource 2 | 205 | 271 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 13049 | 2811 / 43.92 | 2613 |
| timer | 15628 | 4463 / 69.73 | 2907 |
| intake | 2281 | 2258 / 35.28 | 2099 |
| lock | 16399 | 471 / 7.36 | 449 |
| unlock | 16208 | 1805 / 28.20 | 1638 |
| job_end | 12646 | 2056 / 32.12 | 1839 |
| now | 3050417 | 248 / 3.88 | 248 |
| raise | 1368 | 2109 / 32.95 | 2006 |
| dma | 2169 | 710 / 11.09 | 518 |
| fault | 34 | 3364 / 52.56 | 2592 |
| epilogue | 12884 | 1206 / 18.84 | 701 |
| restart | 224 | 4463 / 69.73 | 4158 |
| enforce | 321 | 4463 / 69.73 | 3822 |
| view | 18289 | 321 / 5.02 | 283 |

Entries that interrupted an executing Job: Task 0: 2798, Task 1: 303, Task 2: 13769, Task 3: 486, Task 4: 527, Task 5: 850, Task 6: 543

The last recorded Releases (Task, nominal instant, lag):

- Task 6 at 41000.000 ms, 9362 cycles
- Task 0 at 41010.000 ms, 7946 cycles
- Task 0 at 41020.000 ms, 8221 cycles
- Task 2 at 41020.000 ms, 8221 cycles
- Task 5 at 41025.000 ms, 7754 cycles
- Task 0 at 41030.000 ms, 7798 cycles
- Task 2 at 41040.000 ms, 8883 cycles
- Task 3 at 41040.000 ms, 8883 cycles
- Task 0 at 41050.000 ms, 7938 cycles
- Task 5 at 41050.000 ms, 7938 cycles
- Task 0 at 41060.000 ms, 7986 cycles
- Task 2 at 41060.000 ms, 7986 cycles

## The Kernel's fault record

Each fault the Kernel took (CFSR, and MMFAR, BFAR, or the stacked PC), oldest first:

```
340 faults recorded (128 held)
   0: diagnostic SHCSR = 0x00070008
   1: diagnostic CCR = 0x00000200
   2: diagnostic NVIC_IABR0 = 0x02000000
   3: diagnostic PSP = 0x20010fe0
   4: diagnostic MSP = 0x20007e08
   5: diagnostic frame xPSR = 0x01000000
   6: diagnostic frame PC = 0x000380ba
   7: diagnostic frame LR = 0x00020015
   8: cfsr 0x00040000 (INVPC) at 0x00000000
   9: diagnostic ICSR = 0x00400006
  10: diagnostic SHCSR = 0x00070008
  11: diagnostic CCR = 0x00000200
  12: diagnostic NVIC_IABR0 = 0x02000000
  13: diagnostic PSP = 0x20010fe0
  14: diagnostic MSP = 0x20007e08
  15: diagnostic frame xPSR = 0x01000000
  16: diagnostic frame PC = 0x000380ba
  17: diagnostic frame LR = 0x00020015
  18: cfsr 0x00040000 (INVPC) at 0x00000000
  19: diagnostic ICSR = 0x00400006
  20: diagnostic SHCSR = 0x00070008
  21: diagnostic CCR = 0x00000200
  22: diagnostic NVIC_IABR0 = 0x02000000
  23: diagnostic PSP = 0x20010fe0
  24: diagnostic MSP = 0x20007e08
  25: diagnostic frame xPSR = 0x01000000
  26: diagnostic frame PC = 0x000380ba
  27: diagnostic frame LR = 0x00020015
  28: cfsr 0x00040000 (INVPC) at 0x00000000
  29: diagnostic ICSR = 0x00400006
  30: diagnostic SHCSR = 0x00070008
  31: diagnostic CCR = 0x00000200
  32: diagnostic NVIC_IABR0 = 0x02000000
  33: diagnostic PSP = 0x20010fe0
  34: diagnostic MSP = 0x20007e08
  35: diagnostic frame xPSR = 0x01000000
  36: diagnostic frame PC = 0x000380ba
  37: diagnostic frame LR = 0x00020015
  38: cfsr 0x00040000 (INVPC) at 0x00000000
  39: diagnostic ICSR = 0x00400006
  40: diagnostic SHCSR = 0x00070008
  41: diagnostic CCR = 0x00000200
  42: diagnostic NVIC_IABR0 = 0x02000000
  43: diagnostic PSP = 0x20010fe0
  44: diagnostic MSP = 0x20007e08
  45: diagnostic frame xPSR = 0x01000000
  46: diagnostic frame PC = 0x000380ba
  47: diagnostic frame LR = 0x00020015
  48: cfsr 0x00040000 (INVPC) at 0x00000000
  49: diagnostic ICSR = 0x00400006
  50: diagnostic SHCSR = 0x00070008
  51: diagnostic CCR = 0x00000200
  52: diagnostic NVIC_IABR0 = 0x02000000
  53: diagnostic PSP = 0x20010fe0
  54: diagnostic MSP = 0x20007e08
  55: diagnostic frame xPSR = 0x01000000
  56: diagnostic frame PC = 0x000380ba
  57: diagnostic frame LR = 0x00020015
  58: cfsr 0x00040000 (INVPC) at 0x00000000
  59: diagnostic ICSR = 0x00400006
  60: diagnostic SHCSR = 0x00070008
  61: diagnostic CCR = 0x00000200
  62: diagnostic NVIC_IABR0 = 0x02000000
  63: diagnostic PSP = 0x20010fe0
  64: diagnostic MSP = 0x20007e08
  65: diagnostic frame xPSR = 0x01000000
  66: diagnostic frame PC = 0x000380ba
  67: diagnostic frame LR = 0x00020015
  68: cfsr 0x00040000 (INVPC) at 0x00000000
  69: diagnostic ICSR = 0x00400006
  70: diagnostic SHCSR = 0x00070008
  71: diagnostic CCR = 0x00000200
  72: diagnostic NVIC_IABR0 = 0x02000000
  73: diagnostic PSP = 0x20010fe0
  74: diagnostic MSP = 0x20007e08
  75: diagnostic frame xPSR = 0x01000000
  76: diagnostic frame PC = 0x000380ba
  77: diagnostic frame LR = 0x00020015
  78: cfsr 0x00040000 (INVPC) at 0x00000000
  79: diagnostic ICSR = 0x00400006
  80: diagnostic SHCSR = 0x00070008
  81: diagnostic CCR = 0x00000200
  82: diagnostic NVIC_IABR0 = 0x02000000
  83: diagnostic PSP = 0x20010fe0
  84: diagnostic MSP = 0x20007e08
  85: diagnostic frame xPSR = 0x01000000
  86: diagnostic frame PC = 0x000380ba
  87: diagnostic frame LR = 0x00020015
  88: cfsr 0x00040000 (INVPC) at 0x00000000
  89: diagnostic ICSR = 0x00429006
  90: diagnostic SHCSR = 0x00070008
  91: diagnostic CCR = 0x00000200
  92: diagnostic NVIC_IABR0 = 0x02000000
  93: diagnostic PSP = 0x20010fe0
  94: diagnostic MSP = 0x20007e08
  95: diagnostic frame xPSR = 0x01000000
  96: diagnostic frame PC = 0x000380ba
  97: diagnostic frame LR = 0x00020015
  98: cfsr 0x00040000 (INVPC) at 0x00000000
  99: diagnostic ICSR = 0x00400006
 100: diagnostic SHCSR = 0x00070008
 101: diagnostic CCR = 0x00000200
 102: diagnostic NVIC_IABR0 = 0x02000000
 103: diagnostic PSP = 0x20010fe0
 104: diagnostic MSP = 0x20007e08
 105: diagnostic frame xPSR = 0x01000000
 106: diagnostic frame PC = 0x000380ba
 107: diagnostic frame LR = 0x00020015
 108: cfsr 0x00040000 (INVPC) at 0x00000000
 109: diagnostic ICSR = 0x00400006
 110: diagnostic SHCSR = 0x00070008
 111: diagnostic CCR = 0x00000200
 112: diagnostic NVIC_IABR0 = 0x02000000
 113: diagnostic PSP = 0x20010fe0
 114: diagnostic MSP = 0x20007e08
 115: diagnostic frame xPSR = 0x01000000
 116: diagnostic frame PC = 0x000380ba
 117: diagnostic frame LR = 0x00020015
 118: cfsr 0x00040000 (INVPC) at 0x00000000
 119: diagnostic ICSR = 0x00424006
 120: diagnostic SHCSR = 0x00070008
 121: diagnostic CCR = 0x00000200
 122: diagnostic NVIC_IABR0 = 0x00100000
 123: diagnostic PSP = 0x00000000
 124: diagnostic MSP = 0x20007e58
 125: diagnostic frame xPSR = 0x00000000
 126: diagnostic frame PC = 0x0000023d
 127: diagnostic frame LR = 0x00000101
```

## The Kernel's events

2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 1: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 1 at 140.953 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 1 | 140.953 | overrun | 1 | 0 | 0x5e8e |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 485 | 17 | 0.440 |
| 1 | 66 | 6 | 0.110 |
| 2 | 13548 | 41 | 12.420 |
| 3 | 1560 | 5 | 3.120 |
| 4 | 156 | 10 | 0.240 |
| 5 | 181 | 7 | 0.400 |
| 6 | 61 | 1 | 0.610 |
| none | 2405 | 20 | 5.650 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short_defer_nofix/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short_defer_nofix/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=4104
sensor=2051
log=5
pulse=11
echo=1368
blocked_releases=2051
filter=6
filter_state_bits=1113357148
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1347
loop:part0:sensor:late=3356
loop:part1:spin_ms:spin=0
loop:part1:log:busy=340
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=0
dma_after_stop=0
spim0_enable=7
spim0_shorts=0
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=40418
  detail=37392
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=40418
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=6
  at_ms=40418
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=40514
  detail=37408
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=40514
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=6
  at_ms=40514
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=40617
  detail=37344
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=40617
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=6
  at_ms=40617
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=40715
  detail=37392
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=40715
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=6
  at_ms=40715
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=40817
  detail=37564
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=40817
  detail=6
  prs_lo=0
  prs_hi=0
event=4
  task=4
  at_ms=40856
  detail=0
  prs_lo=4194304
  prs_hi=0
event=16
  task=255
  at_ms=40856
  detail=4
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=40914
  detail=37392
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=40914
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=6
  at_ms=40914
  detail=1
  prs_lo=128
  prs_hi=0
event=7
  task=0
  at_ms=41040
  detail=0
  prs_lo=134217728
  prs_hi=0
event=9
  task=0
  at_ms=41040
  detail=0
  prs_lo=4096
  prs_hi=0
event=6
  task=6
  at_ms=41060
  detail=36984
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=41060
  detail=6
  prs_lo=0
  prs_hi=0
event=4
  task=255
  at_ms=41060
  detail=0
  prs_lo=4194304
  prs_hi=0
event=18
  task=255
  at_ms=41060
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=18000
expected_sensor=8992
expected_late_jobs=8
late_events_seen=7
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0xd9b6 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1171:17
    Frame 1: report_and_exit @ 0xd99a
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/system/src/lib.rs:344:5
    Frame 2: safe_state @ 0xdf38
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/apps/p1-demo/decl/src/nrf52840_timing_short.rs:26:1
    Frame 3: fault<rsk_system::Sys> @ 0x6642
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:693:9
    Frame 4: usage_fault<rsk_system::Sys> @ 0xcd38 inline
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-kernel/src/arch/cm4/mod.rs:762:5
    Frame 5: rsk_usagefault_rust @ 0xcbd8
        /Users/mars2292/repos/Kiro-experimentation/IronOxide/crates/rsk-entry/src/lib.rs:190:5
    Frame 6: rsk_usagefault_entry : ERROR: UNWIND: Tried to unwind `RegisterRule` at CFA = None. @ 0x24c
     Finished in 7.41s
ERROR probe_rs_debug::debug_info: Other("UNWIND: Tried to unwind `RegisterRule` at CFA = None.")
Error: Application exit
```
