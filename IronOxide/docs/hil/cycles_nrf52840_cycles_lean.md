# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:2c909d450de3b4f75f2db8762d3a3507df0122dffc5df625403a6514e3649467 (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 11.5 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 76305 cycles (1.192 ms), log mean 80351 cycles (1.255 ms): difference 4045 cycles (5.30 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 4045 cycles (0.063 ms) per log Job over its 49 Jobs, interrupted by 118 Kernel entries: 1680 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 74512 max 78120, log min 79648 max 81738 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 144007 ticks (576028 cycles) against its largest execution 81738 cycles; control's largest response 40048 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3760 / 58.8 | 3117 | 2^12:199 |
| 2 | 99 | 3404 / 53.2 | 2979 | 2^12:99 |
| 3 | 49 | 2812 / 43.9 | 2812 | 2^12:49 |
| 5 | 79 | 3760 / 58.8 | 3242 | 2^12:79 |
| 6 | 19 | 3392 / 53.0 | 3392 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.503 | 0 | 0 | 0 |
| 2 | 99 | 7.595 | 0 | 0 | 0 |
| 3 | 49 | 9.000 | 0 | 0 | 0 |
| 5 | 80 | 7.731 | 0 | 0 | 0 |
| 6 | 20 | 1.810 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 78120 / 1.221 (min 74512, mean 76305) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 399536 / 6.243 (min 395903, mean 396162) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 81738 / 1.277 (min 79648, mean 80351) | 0 above the Budget; 2^15:49 |
| Task 5 entry | 80 | 8353 / 0.131 (min 4981, mean 5959) | 0 above the Budget; 2^13:60 2^14:20 |
| Task 6 entry | 20 | 5782 / 0.090 (min 5782, mean 5782) | 0 above the Budget; 2^13:20 |
| Task 2 on Resource 0 | 99 | 385062 / 6.017 | |
| Task 2 on Resource 1 | 198 | 218 / 0.003 | |
| Task 5 on Resource 2 | 80 | 246 / 0.004 | |
| Task 6 on Resource 2 | 20 | 227 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 448 | 2403 / 37.55 | 2271 |
| timer | 503 | 3249 / 50.77 | 2730 |
| lock | 397 | 286 / 4.47 | 286 |
| unlock | 397 | 1553 / 24.27 | 1411 |
| job_end | 448 | 1750 / 27.34 | 1537 |
| now | 88091 | 153 / 2.39 | 151 |
| dma | 20 | 533 / 8.33 | 533 |
| epilogue | 448 | 1017 / 15.89 | 507 |
| view | 616 | 327 / 5.11 | 290 |

Entries that interrupted an executing Job: Task 0: 101, Task 2: 204, Task 3: 118, Task 5: 21

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3316 cycles
- Task 0 at 1930.000 ms, 3280 cycles
- Task 0 at 1940.000 ms, 2812 cycles
- Task 2 at 1945.000 ms, 2812 cycles
- Task 0 at 1950.000 ms, 3448 cycles
- Task 5 at 1950.000 ms, 3448 cycles
- Task 0 at 1960.000 ms, 2812 cycles
- Task 2 at 1965.000 ms, 2768 cycles
- Task 0 at 1970.000 ms, 3024 cycles
- Task 5 at 1975.000 ms, 2772 cycles
- Task 0 at 1980.000 ms, 2772 cycles
- Task 0 at 1990.000 ms, 2732 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-lean/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=0
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
     Finished in 6.66s
```
