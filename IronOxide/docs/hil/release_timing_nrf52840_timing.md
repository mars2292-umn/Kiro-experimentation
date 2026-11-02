# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 8000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | RSK-OK after 3669 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6789 cycles (106.1 µs) against J_rel 8000 cycles |
| Kernel events | pass | 1324 events mirrored (1024 held, 3 of them the post-detach reboot's): overrun of Task 2: 462, deadline_miss of Task 2: 139, release_overlap of Task 2: 279, mit_violation of Task 2: 139, operating_end of Task none: 1, safe_state of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 203855 rises and 203334 response edges in the capture); latency 70 to 12540 µs, median 2390 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 71.5 % of the run, the Kernel 53.8 % over 60113401 entries; runs per Task [369218, 122040, 646231, 107113, 203984, 147135, 36806] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 6789 / 106.1 | 3248 | 2^12:364211 2^13:1788 |
| 2 | 182821 | 6789 / 106.1 | 3344 | 2^12:181676 2^13:1145 |
| 3 | 91499 | 6789 / 106.1 | 3467 | 2^12:90800 2^13:699 |
| 5 | 146399 | 6789 / 106.1 | 3283 | 2^12:145602 2^13:797 |
| 6 | 36599 | 6789 / 106.1 | 3735 | 2^12:36197 2^13:402 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3121 cycles
- Task 2 at 3659940.000 ms, 3121 cycles
- Task 0 at 3659950.000 ms, 3405 cycles
- Task 5 at 3659950.000 ms, 3405 cycles
- Task 0 at 3659960.000 ms, 3321 cycles
- Task 2 at 3659960.000 ms, 3321 cycles
- Task 3 at 3659960.000 ms, 3321 cycles
- Task 0 at 3659970.000 ms, 3005 cycles
- Task 5 at 3659975.000 ms, 3261 cycles
- Task 0 at 3659980.000 ms, 3121 cycles
- Task 2 at 3659980.000 ms, 3121 cycles
- Task 0 at 3659990.000 ms, 2941 cycles

## The Kernel's events

1324 events mirrored (1024 held, 3 of them the post-detach reboot's): overrun of Task 2: 462, deadline_miss of Task 2: 139, release_overlap of Task 2: 279, mit_violation of Task 2: 139, operating_end of Task none: 1, safe_state of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 5160828 | 369218 | 0.290 |
| 1 | 410113 | 122040 | 0.190 |
| 2 | 225745277 | 646231 | 12.230 |
| 3 | 27772033 | 107113 | 3.100 |
| 4 | 1289947 | 203984 | 0.160 |
| 5 | 1042972 | 147135 | 0.180 |
| 6 | 289645 | 36806 | 0.180 |
| none | 104285006 | 668122 | 7.450 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182822
log=91500
pulse=203333
echo=121999
blocked_releases=182822
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
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
RSK-OK
Firmware exited successfully
     Finished in 6.70s
```
