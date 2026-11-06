# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:443f178ce32d278f6a30997375496faa53b3ed21293fa3db7a09abb95fbb1023 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.1 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77285 cycles (1.208 ms), log mean 83067 cycles (1.298 ms): difference 5782 cycles (7.48 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5782 cycles (0.090 ms) per log Job over its 49 Jobs, interrupted by 159 Kernel entries: 1782 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74959 max 85232, log min 80633 max 90631 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 149190 ticks (596760 cycles) against its largest execution 90631 cycles; control's largest response 41578 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 5844 / 91.3 | 3401 | 2^12:196 2^13:3 |
| 2 | 99 | 7886 / 123.2 | 3311 | 2^12:98 2^13:1 |
| 3 | 49 | 3158 / 49.3 | 3107 | 2^12:49 |
| 5 | 79 | 4944 / 77.2 | 3515 | 2^12:77 2^13:2 |
| 6 | 19 | 4254 / 66.5 | 3689 | 2^12:18 2^13:1 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.599 | 0 | 0 | 0 |
| 2 | 99 | 7.706 | 0 | 0 | 0 |
| 3 | 49 | 9.324 | 0 | 0 | 0 |
| 4 | 132 | 7.759 | 133 | 2097 | 1952 |
| 5 | 80 | 8.035 | 0 | 0 | 0 |
| 6 | 20 | 1.888 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 85232 / 1.332 (min 74959, mean 77285) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 404484 / 6.320 (min 397904, mean 398576) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 90631 / 1.416 (min 80633, mean 83067) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 132 | 6811 / 0.106 (min 3050, mean 4555) | 0 above the Budget; 2^12:77 2^13:55 |
| Task 5 entry | 80 | 9678 / 0.151 (min 5918, mean 6882) | 0 above the Budget; 2^13:61 2^14:19 |
| Task 6 entry | 20 | 10338 / 0.162 (min 6918, mean 7091) | 0 above the Budget; 2^13:19 2^14:1 |
| Task 2 on Resource 0 | 99 | 391257 / 6.113 | |
| Task 2 on Resource 1 | 198 | 264 / 0.004 | |
| Task 5 on Resource 2 | 80 | 292 / 0.005 | |
| Task 6 on Resource 2 | 20 | 273 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 580 | 2800 / 43.75 | 2656 |
| timer | 684 | 3523 / 55.05 | 2970 |
| intake | 133 | 2165 / 33.83 | 2019 |
| lock | 397 | 463 / 7.23 | 462 |
| unlock | 397 | 1794 / 28.03 | 1640 |
| job_end | 580 | 2004 / 31.31 | 1760 |
| now | 67802 | 239 / 3.73 | 239 |
| dma | 20 | 701 / 10.95 | 699 |
| epilogue | 580 | 1230 / 19.22 | 688 |
| view | 779 | 328 / 5.12 | 299 |

Entries that interrupted an executing Job: Task 0: 127, Task 2: 292, Task 3: 159, Task 4: 55, Task 5: 19, Task 6: 1

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3584 cycles
- Task 0 at 1930.000 ms, 3672 cycles
- Task 0 at 1940.000 ms, 3106 cycles
- Task 2 at 1945.000 ms, 3098 cycles
- Task 0 at 1950.000 ms, 3600 cycles
- Task 5 at 1950.000 ms, 3600 cycles
- Task 0 at 1960.000 ms, 3106 cycles
- Task 2 at 1965.000 ms, 3058 cycles
- Task 0 at 1970.000 ms, 3288 cycles
- Task 5 at 1975.000 ms, 3066 cycles
- Task 0 at 1980.000 ms, 3070 cycles
- Task 0 at 1990.000 ms, 3026 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=132
echo=0
blocked_releases=0
filter=80
filter_state_bits=1120403155
sampler=20
dma_ok=20
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
  at_ms=2000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=2000
  detail=0
  prs_lo=0
  prs_hi=0
RSK-OK
Firmware exited successfully
     Finished in 7.33s
```
