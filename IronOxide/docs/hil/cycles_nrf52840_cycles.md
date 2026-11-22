# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:a63abe327e18059846b70abbd4970dd2131183afee3c09cb896d94d048ec873d (the cycle configuration `nrf52840_cycles`: control and log execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 13.1 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77490 cycles (1.211 ms), log mean 82887 cycles (1.295 ms): difference 5396 cycles (6.96 %) against 10 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5396 cycles (0.084 ms) per log Job over its 49 Jobs, interrupted by 118 Kernel entries: 2241 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75500 max 79544, log min 81902 max 84679 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 148876 ticks (595504 cycles) against its largest execution 84679 cycles; control's largest response 41861 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 4072 / 63.6 | 3368 | 2^12:199 |
| 2 | 99 | 3676 / 57.4 | 3237 | 2^12:99 |
| 3 | 49 | 3059 / 47.8 | 3059 | 2^12:49 |
| 5 | 79 | 3884 / 60.7 | 3466 | 2^12:79 |
| 6 | 19 | 3603 / 56.3 | 3599 | 2^12:19 |

Releases recorded: 445; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.616 | 0 | 0 | 0 |
| 2 | 99 | 7.747 | 0 | 0 | 0 |
| 3 | 49 | 9.305 | 0 | 0 | 0 |
| 5 | 80 | 7.993 | 0 | 0 | 0 |
| 6 | 20 | 1.993 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 79544 / 1.243 (min 75500, mean 77490) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 406231 / 6.347 (min 401635, mean 402221) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 84679 / 1.323 (min 81902, mean 82887) | 0 above the Budget; 2^15:49 |
| Task 5 entry | 80 | 11289 / 0.176 (min 7221, mean 9194) | 0 above the Budget; 2^13:39 2^14:41 |
| Task 6 entry | 20 | 8746 / 0.137 (min 7242, mean 7324) | 0 above the Budget; 2^13:19 2^14:1 |
| Task 2 on Resource 0 | 99 | 386238 / 6.035 | |
| Task 2 on Resource 1 | 198 | 657 / 0.010 | |
| Task 5 on Resource 2 | 80 | 4302 / 0.067 | |
| Task 6 on Resource 2 | 20 | 663 / 0.010 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 448 | 2990 / 46.72 | 2837 |
| timer | 532 | 3346 / 52.28 | 2857 |
| lock | 397 | 458 / 7.16 | 456 |
| unlock | 397 | 1773 / 27.70 | 1613 |
| job_end | 448 | 1984 / 31.00 | 1712 |
| now | 40653 | 227 / 3.55 | 224 |
| dma | 1 | 725 / 11.33 | 725 |
| epilogue | 448 | 1262 / 19.72 | 740 |
| view | 616 | 327 / 5.11 | 290 |

Entries that interrupted an executing Job: Task 0: 101, Task 2: 204, Task 3: 118, Task 5: 41

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 1 (timer), 2874 cycles; Task 2: 3 (dispatch, timer), 8650 cycles; Task 3: 3 (dispatch, timer), 9202 cycles; Task 5: 1 (timer), 2826 cycles; Task 6: none

The last recorded Releases (Task, nominal instant, lag):

- Task 5 at 1925.000 ms, 3600 cycles
- Task 0 at 1930.000 ms, 4072 cycles
- Task 0 at 1940.000 ms, 3051 cycles
- Task 2 at 1945.000 ms, 3043 cycles
- Task 0 at 1950.000 ms, 3884 cycles
- Task 5 at 1950.000 ms, 3884 cycles
- Task 0 at 1960.000 ms, 3059 cycles
- Task 2 at 1965.000 ms, 3023 cycles
- Task 0 at 1970.000 ms, 3540 cycles
- Task 5 at 1975.000 ms, 3039 cycles
- Task 0 at 1980.000 ms, 3031 cycles
- Task 0 at 1990.000 ms, 3003 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles/thumbv7em-none-eabihf/release/p1-demo

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
     Finished in 8.35s
```
