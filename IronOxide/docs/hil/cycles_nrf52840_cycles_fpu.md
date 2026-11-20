# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:9e807a852bedfd5ff84eba69bcbd283941c823cff34a05cf8c7689d989011f2d (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.9 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77980 cycles (1.218 ms), filter mean 83708 cycles (1.308 ms): difference 5728 cycles (7.35 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5728 cycles (0.090 ms) per log Job over its 49 Jobs, interrupted by 134 Kernel entries: 2095 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75429 max 86635, log min 82014 max 90955 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148408 ticks (593632 cycles) against its largest execution 90955 cycles; control's largest response 42835 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 5454 / 85.2 | 3501 | 2^12:198 2^13:1 |
| 2 | 99 | 6634 / 103.7 | 3497 | 2^12:97 2^13:2 |
| 3 | 49 | 3600 / 56.2 | 3541 | 2^12:49 |
| 5 | 49 | 3600 / 56.2 | 3541 | 2^12:49 |
| 6 | 19 | 3548 / 55.4 | 3540 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.677 | 0 | 0 | 0 |
| 2 | 99 | 7.793 | 0 | 0 | 0 |
| 3 | 49 | 10.508 | 0 | 0 | 0 |
| 4 | 131 | 7.850 | 132 | 2264 | 2084 |
| 5 | 49 | 9.275 | 0 | 0 | 0 |
| 6 | 20 | 1.793 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 86635 / 1.354 (min 75429, mean 77980) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 410443 / 6.413 (min 399923, mean 400857) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 84276 / 1.317 (min 75591, mean 76579) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 131 | 13348 / 0.209 (min 3592, mean 5193) | 0 above the Budget; 2^12:78 2^13:52 2^14:1 |
| Task 5 entry | 49 | 90955 / 1.421 (min 82014, mean 83708) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11953 / 0.187 (min 6853, mean 10222) | 0 above the Budget; 2^13:2 2^14:18 |
| Task 2 on Resource 0 | 99 | 386977 / 6.047 | |
| Task 2 on Resource 1 | 198 | 4059 / 0.063 | |
| Task 6 on Resource 2 | 20 | 470 / 0.007 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 548 | 3274 / 51.16 | 3074 |
| timer | 658 | 3387 / 52.92 | 3077 |
| intake | 132 | 2389 / 37.33 | 2197 |
| lock | 317 | 450 / 7.03 | 448 |
| unlock | 317 | 1892 / 29.56 | 1686 |
| job_end | 548 | 2034 / 31.78 | 1776 |
| now | 50004 | 228 / 3.56 | 225 |
| dma | 1 | 735 / 11.48 | 735 |
| epilogue | 548 | 1250 / 19.53 | 693 |
| view | 731 | 329 / 5.14 | 297 |

Entries that interrupted an executing Job: Task 0: 125, Task 2: 287, Task 3: 17, Task 4: 55, Task 5: 134, Task 6: 18

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8638 cycles; Task 2: 5 (dispatch, timer, intake), 14321 cycles; Task 3: 3 (dispatch, timer, intake), 8417 cycles; Task 4: 3 (dispatch, timer), 9185 cycles; Task 5: 5 (dispatch, timer, intake), 15118 cycles; Task 6: 1 (timer), 3018 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3540 cycles
- Task 5 at 1924.000 ms, 3540 cycles
- Task 2 at 1925.000 ms, 3610 cycles
- Task 0 at 1930.000 ms, 3670 cycles
- Task 0 at 1940.000 ms, 3264 cycles
- Task 2 at 1945.000 ms, 3260 cycles
- Task 0 at 1950.000 ms, 3482 cycles
- Task 0 at 1960.000 ms, 3264 cycles
- Task 2 at 1965.000 ms, 3192 cycles
- Task 0 at 1970.000 ms, 3946 cycles
- Task 0 at 1980.000 ms, 3200 cycles
- Task 0 at 1990.000 ms, 3124 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=131
echo=0
blocked_releases=0
filter=49
filter_state_bits=1120384580
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
     Finished in 8.07s
```
