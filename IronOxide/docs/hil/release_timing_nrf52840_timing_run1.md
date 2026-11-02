# HIL release-timing run (task 7.3, R9.10)

Verdict: **FAIL**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 8000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.0 s and reset |
| target run | pass | RSK-OK after 3669 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6690 cycles (104.5 µs) against J_rel 8000 cycles |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | FAIL | 203332 stimulus pulses answered (target counted 203332; 203842 rises and 203333 response edges in the capture); latency 70 to 12500 µs, median 2360 µs, 0 later than 15 ms; 1 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 71.2 % of the run, the Kernel 50.2 % over 20797330 entries; runs per Task [369258, 122042, 645861, 106956, 203873, 147097, 36771] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 6690 / 104.5 | 3083 | 2^12:364641 2^13:1358 |
| 2 | 182821 | 6690 / 104.5 | 3181 | 2^12:181869 2^13:952 |
| 3 | 91499 | 6690 / 104.5 | 3311 | 2^12:90969 2^13:530 |
| 5 | 146399 | 6690 / 104.5 | 3122 | 2^12:145834 2^13:565 |
| 6 | 36599 | 6690 / 104.5 | 3592 | 2^12:36334 2^13:265 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 2906 cycles
- Task 2 at 3659940.000 ms, 2906 cycles
- Task 0 at 3659950.000 ms, 3042 cycles
- Task 5 at 3659950.000 ms, 3042 cycles
- Task 0 at 3659960.000 ms, 3210 cycles
- Task 2 at 3659960.000 ms, 3210 cycles
- Task 3 at 3659960.000 ms, 3210 cycles
- Task 0 at 3659970.000 ms, 3010 cycles
- Task 5 at 3659975.000 ms, 2914 cycles
- Task 0 at 3659980.000 ms, 2950 cycles
- Task 2 at 3659980.000 ms, 2950 cycles
- Task 0 at 3659990.000 ms, 2822 cycles

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 4718302 | 369258 | 0.250 |
| 1 | 378246 | 122042 | 0.080 |
| 2 | 225329509 | 645861 | 12.210 |
| 3 | 27739032 | 106956 | 3.110 |
| 4 | 1186350 | 203873 | 0.150 |
| 5 | 950585 | 147097 | 0.170 |
| 6 | 263417 | 36771 | 0.140 |
| none | 105430481 | 668809 | 9.800 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=365999
sensor=182821
log=91500
pulse=203332
echo=121999
blocked_releases=182821
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
     Finished in 6.40s
```
