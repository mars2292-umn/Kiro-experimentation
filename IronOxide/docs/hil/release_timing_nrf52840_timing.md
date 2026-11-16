# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3672 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8742 cycles (136.6 µs) against J_rel 12500 cycles |
| Kernel events | pass | 984 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 2: 78, deadline_miss of Task 0: 1, deadline_miss of Task 2: 177, release_overlap of Task 2: 178, mit_violation of Task 2: 78; the trace froze at event 983 (deadline_miss of Task 0 at 3075040.025 ms); the deliberate overload case: 1 deadline miss(es) of Task 0 (control) after the injected late Job of Task 2 (sensor), whose 12 ms Critical_Section then blocks control for nearly its 10 ms deadline; the trace and the mirror stop at the first, so the events after it are not judged by this step |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 11 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 983 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 20344 cycles (317.9 µs) over 213477566 spans, closed by a timer entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 204001 rises and 203334 response edges in the capture); latency 90 to 12640 µs, median 2470 µs, 0 later than 15 ms; Δ_stamp observed 6768 cycles (105.8 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.6 % of the run, the Kernel 59.3 % over 37242199 entries; runs per Task [369386, 122035, 647852, 106961, 204218, 147360, 36926] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24079 cycles (0.376 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23040 cycles (0.360 ms at 100000 samples/s; 369386 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 1021 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 2801 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 256 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 8742 / 136.6 | 3741 | 2^12:335226 2^13:30771 2^14:2 |
| 2 | 182821 | 8742 / 136.6 | 3750 | 2^12:162507 2^13:20313 2^14:1 |
| 3 | 91499 | 8742 / 136.6 | 3890 | 2^12:72179 2^13:19318 2^14:2 |
| 5 | 146399 | 8398 / 131.2 | 3818 | 2^12:117819 2^13:28579 2^14:1 |
| 6 | 36599 | 8398 / 131.2 | 4184 | 2^12:17986 2^13:18612 2^14:1 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 10.156 | 0 | 0 | 0 |
| 1 | 121999 | 0.596 | 0 | 0 | 0 |
| 2 | 182822 | 54.660 | 0 | 0 | 0 |
| 3 | 91500 | 7.123 | 0 | 0 | 0 |
| 4 | 203333 | 12.653 | 203334 | 2364 | 2150 |
| 5 | 146400 | 11.493 | 0 | 0 | 0 |
| 6 | 36600 | 57.154 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 24079 / 0.376 (min 9935, mean 12964) | 0 above the Budget; 2^14:319627 2^15:46373 |
| Task 1 entry | 121999 | 15506 / 0.242 (min 3402, mean 3552) | 0 above the Budget; 2^12:121191 2^13:806 2^14:2 |
| Task 2 entry | 182822 | 2948017 / 46.063 (min 783900, mean 789780) | 178 above the Budget; 2^15:182822 |
| Task 3 entry | 91500 | 205209 / 3.206 (min 180777, mean 195568) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 18315 / 0.286 (min 3426, mean 3880) | 0 above the Budget; 2^12:188834 2^13:13853 2^14:639 2^15:7 |
| Task 5 entry | 146400 | 15794 / 0.247 (min 6724, mean 7115) | 0 above the Budget; 2^13:139798 2^14:6602 |
| Task 6 entry | 36600 | 26393 / 0.412 (min 7698, mean 8011) | 0 above the Budget; 2^13:35117 2^14:1464 2^15:19 |
| Task 0 on Resource 0 | 366000 | 8064 / 0.126 | |
| Task 0 on Resource 1 | 366000 | 8099 / 0.127 | |
| Task 2 on Resource 0 | 182822 | 776170 / 12.128 | |
| Task 2 on Resource 1 | 365644 | 6650 / 0.104 | |
| Task 5 on Resource 2 | 146400 | 5662 / 0.088 | |
| Task 6 on Resource 2 | 36600 | 9076 / 0.142 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148657 | 3245 / 50.70 | 3029 |
| timer | 1038957 | 4568 / 71.38 | 3104 |
| intake | 203334 | 2474 / 38.66 | 2258 |
| lock | 1463466 | 473 / 7.39 | 450 |
| unlock | 1463466 | 1932 / 30.19 | 1713 |
| job_end | 1148654 | 2189 / 34.20 | 1879 |
| now | 208729423 | 222 / 3.47 | 221 |
| raise | 122000 | 2199 / 34.36 | 2060 |
| dma | 36608 | 715 / 11.17 | 491 |
| epilogue | 1148654 | 1253 / 19.58 | 753 |
| enforce | 178 | 3060 / 47.81 | 3000 |
| view | 1631233 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 180700, Task 1: 811, Task 2: 1089290, Task 3: 49157, Task 4: 15439, Task 5: 7708, Task 6: 1817

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3500 cycles
- Task 2 at 3659940.000 ms, 3500 cycles
- Task 0 at 3659950.000 ms, 3717 cycles
- Task 5 at 3659950.000 ms, 3717 cycles
- Task 0 at 3659960.000 ms, 3684 cycles
- Task 2 at 3659960.000 ms, 3684 cycles
- Task 3 at 3659960.000 ms, 3684 cycles
- Task 0 at 3659970.000 ms, 3777 cycles
- Task 5 at 3659975.000 ms, 3545 cycles
- Task 0 at 3659980.000 ms, 3452 cycles
- Task 2 at 3659980.000 ms, 3452 cycles
- Task 0 at 3659990.000 ms, 3521 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 213477566 of them, the longest 20344 cycles (317.9 µs), closed by a timer entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 208248916 |
| 512 to 1023 (8.0 to 16.0 µs) | 1505176 |
| 1024 to 2047 (16.0 to 32.0 µs) | 855869 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1394418 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1161668 |
| 8192 to 16383 (128.0 to 256.0 µs) | 311343 |
| 16384 to 32767 (256.0 to 512.0 µs) | 176 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 985147 | 18397 / 287.5 |
| timer | 920722 | 20344 / 317.9 |
| job_end | 211121208 | 5805 / 90.7 |
| epilogue | 450489 | 7512 / 117.4 |

## The Kernel's events

984 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 2: 78, deadline_miss of Task 0: 1, deadline_miss of Task 2: 177, release_overlap of Task 2: 178, mit_violation of Task 2: 78; the trace froze at event 983 (deadline_miss of Task 0 at 3075040.025 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 983 | 3075040.025 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6904135 | 369386 | 0.360 |
| 1 | 513299 | 122035 | 0.160 |
| 2 | 227824294 | 647852 | 12.300 |
| 3 | 27899211 | 106961 | 3.170 |
| 4 | 932365 | 204218 | 0.190 |
| 5 | 1401430 | 147360 | 0.230 |
| 6 | 396983 | 36926 | 0.290 |
| none | 100123357 | 666069 | 7.260 |

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
blocked_releases=182861
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1021
loop:part0:sensor:late=2801
loop:part1:spin_ms:spin=0
loop:part1:log:busy=256
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
RSK-OK
Firmware exited successfully
     Finished in 8.11s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.echo` (19383 cycles) exceeds its Budget (19200 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
