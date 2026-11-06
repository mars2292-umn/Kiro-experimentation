# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:e04a75ce274557a379df965cf1d4be66da6ab2d322bda05cf86bab03973368bb (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.7 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77234 cycles (1.207 ms), log mean 83034 cycles (1.297 ms): difference 5801 cycles (7.51 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5801 cycles (0.091 ms) per log Job over its 49 Jobs, interrupted by 160 Kernel entries: 1776 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74896 max 85074, log min 80556 max 90418 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148876 ticks (595504 cycles) against its largest execution 90418 cycles; control's largest response 41457 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 6161 / 96.3 | 3466 | 2^12:188 2^13:11 |
| 2 | 99 | 7628 / 119.2 | 3312 | 2^12:98 2^13:1 |
| 3 | 49 | 3152 / 49.2 | 3105 | 2^12:49 |
| 5 | 79 | 4713 / 73.6 | 3555 | 2^12:69 2^13:10 |
| 6 | 19 | 4384 / 68.5 | 3694 | 2^12:18 2^13:1 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.591 | 0 | 0 | 0 |
| 2 | 99 | 7.696 | 0 | 0 | 0 |
| 3 | 49 | 9.305 | 0 | 0 | 0 |
| 4 | 132 | 7.746 | 133 | 1984 | 1844 |
| 5 | 80 | 8.018 | 0 | 0 | 0 |
| 6 | 20 | 1.876 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 85074 / 1.329 (min 74896, mean 77234) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 403963 / 6.312 (min 397555, mean 398238) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 90418 / 1.413 (min 80556, mean 83034) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 132 | 6753 / 0.106 (min 2991, mean 4498) | 0 above the Budget; 2^12:77 2^13:55 |
| Task 5 entry | 80 | 9523 / 0.149 (min 5767, mean 6729) | 0 above the Budget; 2^13:61 2^14:19 |
| Task 6 entry | 20 | 10207 / 0.159 (min 6783, mean 6955) | 0 above the Budget; 2^13:19 2^14:1 |
| Task 2 on Resource 0 | 99 | 390655 / 6.104 | |
| Task 2 on Resource 1 | 198 | 269 / 0.004 | |
| Task 5 on Resource 2 | 80 | 297 / 0.005 | |
| Task 6 on Resource 2 | 20 | 278 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 580 | 2751 / 42.98 | 2602 |
| timer | 687 | 3561 / 55.64 | 2975 |
| intake | 133 | 2058 / 32.16 | 1917 |
| lock | 397 | 456 / 7.12 | 455 |
| unlock | 397 | 1676 / 26.19 | 1542 |
| job_end | 580 | 1901 / 29.70 | 1670 |
| now | 67812 | 237 / 3.70 | 236 |
| dma | 20 | 716 / 11.19 | 716 |
| epilogue | 580 | 1213 / 18.95 | 671 |
| view | 779 | 327 / 5.11 | 298 |

Entries that interrupted an executing Job: Task 0: 128, Task 2: 292, Task 3: 160, Task 4: 55, Task 5: 19, Task 6: 1

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3593 cycles
- Task 0 at 1930.000 ms, 3545 cycles
- Task 0 at 1940.000 ms, 3108 cycles
- Task 2 at 1945.000 ms, 3108 cycles
- Task 0 at 1950.000 ms, 3937 cycles
- Task 5 at 1950.000 ms, 3937 cycles
- Task 0 at 1960.000 ms, 3108 cycles
- Task 2 at 1965.000 ms, 3064 cycles
- Task 0 at 1970.000 ms, 3713 cycles
- Task 5 at 1975.000 ms, 3064 cycles
- Task 0 at 1980.000 ms, 3072 cycles
- Task 0 at 1990.000 ms, 3028 cycles


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
     Finished in 6.90s
```
