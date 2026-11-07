# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7600 cycles (118.8 µs) against J_rel 10000 cycles |
| Kernel events | pass | 54 events mirrored (54 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 16688 cycles (260.8 µs) over 11330256 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10633 rises and 10000 response edges in the capture); latency 90 to 12450 µs, median 2540 µs, 0 later than 15 ms; Δ_stamp observed 3896 cycles (60.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.5 % of the run, the Kernel 63.3 % over 982287 entries; runs per Task [18124, 5999, 31825, 5175, 10000, 7200, 1812] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20087 cycles (0.314 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18124 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 1104 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 3011 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 277 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7600 / 118.8 | 3759 | 2^12:16640 2^13:1359 |
| 2 | 8991 | 7600 / 118.8 | 3784 | 2^12:7976 2^13:1015 |
| 3 | 4499 | 7600 / 118.8 | 3924 | 2^12:3586 2^13:913 |
| 5 | 7199 | 5324 / 83.2 | 3814 | 2^12:5860 2^13:1339 |
| 6 | 1799 | 5324 / 83.2 | 4226 | 2^12:796 2^13:1003 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.882 | 0 | 0 | 0 |
| 1 | 5999 | 0.253 | 0 | 0 | 0 |
| 2 | 8992 | 54.471 | 0 | 0 | 0 |
| 3 | 4500 | 6.992 | 0 | 0 | 0 |
| 4 | 9999 | 12.465 | 10000 | 2230 | 2056 |
| 5 | 7200 | 11.387 | 0 | 0 | 0 |
| 6 | 1800 | 56.847 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20087 / 0.314 (min 9591, mean 12447) | 0 above the Budget; 2^14:17773 2^15:227 |
| Task 1 entry | 5999 | 7030 / 0.110 (min 3158, mean 3252) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8992 | 2955281 / 46.176 (min 783217, mean 788724) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201591 / 3.150 (min 181392, mean 195655) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10913 / 0.171 (min 3247, mean 3470) | 0 above the Budget; 2^12:9676 2^13:316 2^14:7 |
| Task 5 entry | 7200 | 10578 / 0.165 (min 6418, mean 6604) | 0 above the Budget; 2^13:7147 2^14:53 |
| Task 6 entry | 1800 | 27634 / 0.432 (min 7354, mean 9919) | 0 above the Budget; 2^13:569 2^14:1229 2^15:2 |
| Task 0 on Resource 0 | 18000 | 4273 / 0.067 | |
| Task 0 on Resource 1 | 18000 | 4242 / 0.066 | |
| Task 2 on Resource 0 | 8992 | 777094 / 12.142 | |
| Task 2 on Resource 1 | 17984 | 4099 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 4289 / 0.067 | |
| Task 6 on Resource 2 | 1800 | 5200 / 0.081 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2884 / 45.06 | 2750 |
| timer | 52231 | 4512 / 70.50 | 3149 |
| intake | 10000 | 2358 / 36.84 | 2182 |
| lock | 71976 | 454 / 7.09 | 452 |
| unlock | 71976 | 1879 / 29.36 | 1677 |
| job_end | 56490 | 2169 / 33.89 | 1877 |
| now | 11096321 | 235 / 3.67 | 235 |
| raise | 6000 | 2152 / 33.62 | 2034 |
| dma | 1808 | 721 / 11.27 | 503 |
| epilogue | 56490 | 1228 / 19.19 | 727 |
| enforce | 8 | 3080 / 48.12 | 3039 |
| view | 80014 | 330 / 5.16 | 293 |

Entries that interrupted an executing Job: Task 0: 8927, Task 1: 3, Task 2: 53926, Task 3: 2385, Task 4: 330, Task 5: 53, Task 6: 1250

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3541 cycles
- Task 2 at 179940.000 ms, 3541 cycles
- Task 0 at 179950.000 ms, 3987 cycles
- Task 5 at 179950.000 ms, 3987 cycles
- Task 0 at 179960.000 ms, 3829 cycles
- Task 2 at 179960.000 ms, 3829 cycles
- Task 3 at 179960.000 ms, 3829 cycles
- Task 0 at 179970.000 ms, 3783 cycles
- Task 5 at 179975.000 ms, 3695 cycles
- Task 0 at 179980.000 ms, 3541 cycles
- Task 2 at 179980.000 ms, 3541 cycles
- Task 0 at 179990.000 ms, 3475 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 11330256 of them, the longest 16688 cycles (260.8 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 11071862 |
| 512 to 1023 (8.0 to 16.0 µs) | 73780 |
| 1024 to 2047 (16.0 to 32.0 µs) | 49853 |
| 2048 to 4095 (32.0 to 64.0 µs) | 60183 |
| 4096 to 8191 (64.0 to 128.0 µs) | 59326 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15251 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48907 | 16688 / 260.8 |
| timer | 46018 | 14778 / 230.9 |
| job_end | 11212823 | 2386 / 37.3 |
| epilogue | 22508 | 3959 / 61.9 |

## The Kernel's events

54 events mirrored (54 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 330720 | 18124 | 0.310 |
| 1 | 24333 | 5999 | 0.100 |
| 2 | 11182470 | 31825 | 12.290 |
| 3 | 1373285 | 5175 | 3.130 |
| 4 | 41818 | 10000 | 0.160 |
| 5 | 64862 | 7200 | 0.160 |
| 6 | 25460 | 1812 | 0.260 |
| none | 4953353 | 32960 | 7.230 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8992
log=4500
pulse=9999
echo=5999
blocked_releases=8993
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1104
loop:part0:sensor:late=3011
loop:part1:spin_ms:spin=0
loop:part1:log:busy=277
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
late_events_seen=30
RSK-OK
Firmware exited successfully
     Finished in 7.75s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3694102 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
