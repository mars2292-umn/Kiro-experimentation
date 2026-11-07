# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:e8472b184293e59e377614dc71ed56aa0ac456e2c9df75375ad91b4a99ecea19 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.6 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77393 cycles (1.209 ms), log mean 82688 cycles (1.292 ms): difference 5295 cycles (6.84 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5295 cycles (0.083 ms) per log Job over its 49 Jobs, interrupted by 150 Kernel entries: 1730 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74974 max 82503, log min 80675 max 87313 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148426 ticks (593704 cycles) against its largest execution 87313 cycles; control's largest response 40896 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3967 / 62.0 | 3412 | 2^12:199 |
| 2 | 99 | 3663 / 57.2 | 3283 | 2^12:99 |
| 3 | 49 | 3164 / 49.4 | 3128 | 2^12:49 |
| 5 | 79 | 3967 / 62.0 | 3507 | 2^12:79 |
| 6 | 19 | 3696 / 57.8 | 3661 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.556 | 0 | 0 | 0 |
| 2 | 99 | 7.664 | 0 | 0 | 0 |
| 3 | 49 | 9.277 | 0 | 0 | 0 |
| 4 | 110 | 6.455 | 111 | 2096 | 1980 |
| 5 | 80 | 7.987 | 0 | 0 | 0 |
| 6 | 20 | 1.888 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82503 / 1.289 (min 74974, mean 77393) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 402185 / 6.284 (min 397968, mean 398388) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 87313 / 1.364 (min 80675, mean 82688) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 6772 / 0.106 (min 3031, mean 4099) | 0 above the Budget; 2^12:78 2^13:32 |
| Task 5 entry | 80 | 13093 / 0.205 (min 5853, mean 6826) | 0 above the Budget; 2^13:62 2^14:18 |
| Task 6 entry | 20 | 6884 / 0.108 (min 6860, mean 6866) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 388667 / 6.073 | |
| Task 2 on Resource 1 | 198 | 264 / 0.004 | |
| Task 5 on Resource 2 | 80 | 290 / 0.005 | |
| Task 6 on Resource 2 | 20 | 273 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 558 | 2824 / 44.12 | 2701 |
| timer | 647 | 3488 / 54.50 | 2990 |
| intake | 111 | 2219 / 34.67 | 2104 |
| lock | 397 | 463 / 7.23 | 461 |
| unlock | 397 | 1803 / 28.17 | 1609 |
| job_end | 558 | 1989 / 31.08 | 1730 |
| now | 67875 | 242 / 3.78 | 240 |
| dma | 20 | 703 / 10.98 | 703 |
| epilogue | 558 | 1214 / 18.97 | 672 |
| view | 753 | 330 / 5.16 | 301 |

Entries that interrupted an executing Job: Task 0: 135, Task 2: 274, Task 3: 150, Task 4: 32, Task 5: 19

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3587 cycles
- Task 0 at 1930.000 ms, 3651 cycles
- Task 0 at 1940.000 ms, 3116 cycles
- Task 2 at 1945.000 ms, 3116 cycles
- Task 0 at 1950.000 ms, 3703 cycles
- Task 5 at 1950.000 ms, 3703 cycles
- Task 0 at 1960.000 ms, 3116 cycles
- Task 2 at 1965.000 ms, 3080 cycles
- Task 0 at 1970.000 ms, 3403 cycles
- Task 5 at 1975.000 ms, 3072 cycles
- Task 0 at 1980.000 ms, 3080 cycles
- Task 0 at 1990.000 ms, 3096 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=110
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
     Finished in 7.03s
```
