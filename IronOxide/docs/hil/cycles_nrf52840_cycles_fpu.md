# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:7107bc3df75cb35d0728869b8e643cc87dbf300fc64768a39262fe5bb5c6ea43 (the cycle configuration `nrf52840_cycles_fpu`: control and filter (the FPU Task) execute the same 8000-iteration spin per Job).

## Steps

| Step | Result | Detail |
|---|---|---|
| target run | pass | RSK-OK after 12.5 s (operating duration 2 s) |
| Jobs recorded | pass | control 200 Jobs, log 49 Jobs (at least 150 and 40 expected over 2 s) |
| executions agree (R18.1: the preempted Job is charged its own cycles only) | pass | control mean 77452 cycles (1.210 ms), filter mean 82654 cycles (1.291 ms): difference 5202 cycles (6.72 %) against 15 % |
| attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points) | pass | 5202 cycles (0.081 ms) per log Job over its 49 Jobs, interrupted by 130 Kernel entries: 1961 cycles per entry, exported as Δ_entry (R13.6) |
| executions tight (every Job of a Task within a narrow band) | pass | control min 75030 max 82187, log min 81331 max 86178 cycles |
| log preempted (its response times exceed its executions) | pass | log's largest response 146825 ticks (587300 cycles) against its largest execution 86178 cycles; control's largest response 41563 ticks |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 199 | 3924 / 61.3 | 3404 | 2^12:199 |
| 2 | 99 | 3572 / 55.8 | 3371 | 2^12:99 |
| 3 | 49 | 3558 / 55.6 | 3511 | 2^12:49 |
| 5 | 49 | 3558 / 55.6 | 3511 | 2^12:49 |
| 6 | 19 | 3566 / 55.7 | 3515 | 2^12:19 |

Releases recorded: 415; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 200 | 2.598 | 0 | 0 | 0 |
| 2 | 99 | 7.709 | 0 | 0 | 0 |
| 3 | 49 | 10.400 | 0 | 0 | 0 |
| 4 | 110 | 6.628 | 110 | 2146 | 2024 |
| 5 | 49 | 9.177 | 0 | 0 | 0 |
| 6 | 20 | 1.766 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 200 | 82187 / 1.284 (min 75030, mean 77452) | 0 above the Budget; 2^15:200 |
| Task 2 entry | 99 | 403727 / 6.308 (min 399286, mean 400505) | 0 above the Budget; 2^15:99 |
| Task 3 entry | 49 | 78811 / 1.231 (min 75147, mean 75618) | 0 above the Budget; 2^15:49 |
| Task 4 entry | 110 | 7096 / 0.111 (min 3119, mean 4813) | 0 above the Budget; 2^12:62 2^13:48 |
| Task 5 entry | 49 | 86178 / 1.347 (min 81331, mean 82654) | 0 above the Budget; 2^15:49 |
| Task 6 entry | 20 | 11162 / 0.174 (min 7454, mean 10354) | 0 above the Budget; 2^13:4 2^14:16 |
| Task 2 on Resource 0 | 99 | 389396 / 6.084 | |
| Task 2 on Resource 1 | 198 | 402 / 0.006 | |
| Task 6 on Resource 2 | 20 | 411 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 527 | 2805 / 43.83 | 2668 |
| timer | 638 | 3321 / 51.89 | 3055 |
| intake | 110 | 2266 / 35.41 | 2142 |
| lock | 317 | 471 / 7.36 | 469 |
| unlock | 317 | 1850 / 28.91 | 1668 |
| job_end | 527 | 1964 / 30.69 | 1766 |
| now | 54157 | 237 / 3.70 | 235 |
| dma | 20 | 716 / 11.19 | 716 |
| epilogue | 527 | 1184 / 18.50 | 658 |
| view | 702 | 329 / 5.14 | 296 |

Entries that interrupted an executing Job: Task 0: 124, Task 2: 287, Task 3: 6, Task 4: 48, Task 5: 130, Task 6: 16

The last recorded Releases (Task, nominal instant, lag):

- Task 3 at 1924.000 ms, 3498 cycles
- Task 5 at 1924.000 ms, 3498 cycles
- Task 2 at 1925.000 ms, 3504 cycles
- Task 0 at 1930.000 ms, 3856 cycles
- Task 0 at 1940.000 ms, 3222 cycles
- Task 2 at 1945.000 ms, 3294 cycles
- Task 0 at 1950.000 ms, 3464 cycles
- Task 0 at 1960.000 ms, 3222 cycles
- Task 2 at 1965.000 ms, 3158 cycles
- Task 0 at 1970.000 ms, 3716 cycles
- Task 0 at 1980.000 ms, 3218 cycles
- Task 0 at 1990.000 ms, 3118 cycles


## Commands

    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/cycles-fpu/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=200
sensor=99
log=49
pulse=110
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
     Finished in 7.71s
```
